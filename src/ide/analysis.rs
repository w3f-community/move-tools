use lsp_types::Diagnostic;
use move_lang::parser::ast::FileDefinition;

use crate::compiler::{check, parse_file};
use crate::ide::db::{AnalysisChange, FilePath, RootDatabase};

#[derive(Debug, Default)]
pub struct AnalysisHost {
    db: RootDatabase,
}

impl AnalysisHost {
    pub fn analysis(&self) -> Analysis {
        Analysis {
            db: self.db.clone(),
        }
    }

    pub fn db(&self) -> &RootDatabase {
        &self.db
    }

    pub fn apply_change(&mut self, change: AnalysisChange) {
        self.db.apply_change(change);
    }
}

#[derive(Debug)]
pub struct Analysis {
    db: RootDatabase,
}

impl Analysis {
    pub fn parse(&self, fpath: FilePath, text: &str) -> Result<FileDefinition, Vec<Diagnostic>> {
        parse_file(fpath, text).map_err(|err| vec![self.db.libra_error_into_diagnostic(err)])
    }

    pub fn check_with_libra_compiler(&self, fpath: FilePath, text: &str) -> Vec<Diagnostic> {
        match self.check_with_libra_compiler_inner(fpath, text) {
            Ok(_) => vec![],
            Err(ds) => ds,
        }
    }

    fn check_with_libra_compiler_inner(
        &self,
        fpath: FilePath,
        text: &str,
    ) -> Result<(), Vec<Diagnostic>> {
        let main_file = self.parse(fpath, text)?;

        let mut dependencies = vec![];
        for (existing_mod_fpath, existing_mod_text) in self.db.project_files_mapping.iter() {
            if existing_mod_fpath != &fpath {
                let parsed = self.parse(existing_mod_fpath, existing_mod_text)?;
                if matches!(parsed, FileDefinition::Modules(_)) {
                    dependencies.push(parsed);
                }
            }
        }
        let check_res =
            check::check_parsed_program(main_file, dependencies, Some(self.db.sender_address));
        check_res.map_err(|errors| {
            errors
                .into_iter()
                .map(|err| self.db.libra_error_into_diagnostic(err))
                .collect()
        })
    }
}