#![allow(async_fn_in_trait)]

mod base;
pub mod chat;

pub use base::Interactor;
pub use chat::{SaveChat, SaveChatInput};
