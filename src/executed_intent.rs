use log::info;
use tabled::{Style, Table, Tabled};

use crate::intent::{Intent, IntentType};

#[derive(Tabled)]
pub struct ExecutedIntent {
    #[tabled(display_with("Self::display_intent", args))]
    pub intent: IntentType,
    pub subvolume: String,
    pub target: String,
    pub name: String,
    pub success: bool,
}

impl ExecutedIntent {
    pub fn new(intent: &Intent, success: bool) -> Self {
        Self {
            intent: intent.intent.clone(),
            subvolume: intent.subvolume.clone(),
            target: intent.target.clone(),
            name: intent.name.clone(),
            success,
        }
    }

    fn display_intent(&self) -> String {
        match self.intent {
            IntentType::Create => "++++++".to_string(),
            IntentType::Keep => "======".to_string(),
            IntentType::Delete => "------".to_string(),
        }
    }

    pub fn print_tabled(intents: &[Self]) {
        let table = Table::new(intents).with(Style::modern()).to_string();
        info!("{}", table);
    }
}
