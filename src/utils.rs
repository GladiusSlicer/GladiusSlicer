use gladius_shared::error::SlicerErrors;
use gladius_shared::messages::Message;
use log::{debug, error, info};

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
    let cv_message = Message::Error(error);
    println!("{}", serde_json::to_string(&cv_message).unwrap());
}


pub fn display_state_update(state_message: &str , send_message: bool) {
    if send_message{
        let message = Message::StateUpdate(state_message.to_string());
        println!("{}", serde_json::to_string(&message).unwrap());
    }else{
        info!("{}",state_message);
    }
}