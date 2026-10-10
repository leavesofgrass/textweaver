//! Dispatch for the Phase 2 authoring and navigation actions (Agent P2b),
//! so the app's core dispatch stays one arm long for them.

use textweaver_core::Direction;
use textweaver_keymap::ActionId;

use crate::app::App;
use crate::command::{Effect, PromptPurpose};
use crate::publish::OutputFormat;
use crate::tables::TableStep;

impl App {
    /// Runs one of the authoring and navigation actions.
    pub(crate) fn authoring_action(&mut self, a: ActionId) -> Vec<Effect> {
        use ActionId as A;
        match a {
            A::Outline => return self.outline(),
            A::ListenRendered => return self.listen_rendered(),
            A::FollowLink => return self.follow_link(),
            A::TableNextRow => self.table_move(TableStep::Row, Direction::Forward),
            A::TablePreviousRow => self.table_move(TableStep::Row, Direction::Backward),
            A::TableNextColumn => self.table_move(TableStep::Column, Direction::Forward),
            A::TablePreviousColumn => self.table_move(TableStep::Column, Direction::Backward),
            A::CycleVerbosity => self.cycle_verbosity(),
            A::CyclePunctuation => self.cycle_punctuation(),
            A::NextMisspelling => self.misspelling_step(Direction::Forward),
            A::PreviousMisspelling => self.misspelling_step(Direction::Backward),
            A::SpellingSuggestions => return self.spelling_suggestions(),
            A::NextGrammarProblem => self.grammar_action(Direction::Forward),
            A::PreviousGrammarProblem => self.grammar_action(Direction::Backward),
            A::NextLintProblem => self.lint_action(Direction::Forward),
            A::PreviousLintProblem => self.lint_action(Direction::Backward),
            A::ExportStudySheet => return self.export_study_sheet(false),
            A::SelfTest => return self.self_test(),
            A::ExportKnowledgeGraph => return self.export_knowledge_graph(),
            A::NewFromTemplate => return self.new_from_template(),
            A::ExportHtml => return self.export_to(OutputFormat::Html),
            A::ExportPdf => return self.export_to(OutputFormat::Pdf),
            A::ExportDocx => return self.export_to(OutputFormat::Docx),
            A::ExportEpub => return self.export_to(OutputFormat::Epub),
            A::ExportBrf => return self.export_to(OutputFormat::Brf),
            A::PreviewInBrowser => return self.preview_in_browser(),
            A::CyclePreviewFollow => self.cycle_preview_follow(),
            A::SelectAll => self.select_all(),
            A::DeleteWordBefore => return self.delete_word(Direction::Backward),
            A::DeleteWordAfter => return self.delete_word(Direction::Forward),
            A::Paste => return self.paste_from_clipboard(false),
            A::PastePlainText => return self.paste_from_clipboard(true),
            A::InsertCitation => return self.insert_citation(),
            A::AddReference => return self.prompt(PromptPurpose::ReferenceIdentifier),
            A::InsertBibliography => return self.insert_bibliography(),
            A::CheckCitations => self.check_citations(),
            A::ImportReferences => return self.prompt(PromptPurpose::ImportReferences),
            _ => {}
        }
        vec![Effect::Redraw]
    }

    /// The next or previous Markdown lint problem (the `lint` feature).
    fn lint_action(&mut self, dir: Direction) {
        #[cfg(feature = "lint")]
        self.lint_step(dir);
        #[cfg(not(feature = "lint"))]
        {
            let _ = dir;
            self.tell("Markdown lint is not in this build.");
        }
    }

    /// The answers of the authoring prompts.
    pub(crate) fn answer_authoring(&mut self, purpose: PromptPurpose, text: &str) -> Vec<Effect> {
        match purpose {
            PromptPurpose::CitationLocator => self.answer_locator(text),
            PromptPurpose::ReferenceIdentifier => self.answer_identifier(text),
            PromptPurpose::ImportReferences => self.answer_import_references(text),
            PromptPurpose::TemplateTitle => self.answer_template_title(text),
            _ => vec![Effect::Redraw],
        }
    }
}
