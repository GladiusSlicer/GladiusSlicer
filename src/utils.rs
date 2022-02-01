use gladius_shared::error::SlicerErrors;
use gladius_shared::messages::Message;
use log::{error, info};
use std::io::{BufWriter, Write};

pub fn show_error_message(error: SlicerErrors) {
    let (error_code, message) = error.get_code_and_message();
    error!("\n");
    error!("**************************************************");
    error!("\tGladius Slicer Ran into an error");
    error!("\tError Code: {:#X}", error_code);
    error!("\t{}", message);
    error!("**************************************************");
    error!("\n\n\n");
}
pub fn send_error_message(error: SlicerErrors) {
    let message = Message::Error(error);
    bincode::serialize_into(BufWriter::new(std::io::stdout()), &message).unwrap();
    std::io::stdout()
        .flush()
        .expect("Standard Out should be limited");
}

pub fn display_state_update(state_message: &str, send_message: bool) {
    if send_message {
        let message = Message::StateUpdate(state_message.to_string());
        bincode::serialize_into(std::io::stdout(), &message).unwrap();
        std::io::stdout()
            .flush()
            .expect("Standard Out should be limited");
    } else {
        info!("{}", state_message);
    }
}
