use log::{debug,error};
use gladius_shared::error::SlicerErrors;
use gladius_shared::messages::Message;

pub fn show_error_message(error : SlicerErrors) {
    let (error_code, message) = error.get_code_and_message();
    error!("\n");
    error!("**************************************************");
    error!("\tGladius Slicer Ran into an error");
    error!("\tError Code: {:#X}", error_code);
    error!("\t{}", message);
    error!("**************************************************");
    error!("\n\n\n");
}
pub fn send_error_message(error : SlicerErrors) {
    let cv_message = Message::Error(error);
    println!("{}",serde_json::to_string(&cv_message).unwrap());
}