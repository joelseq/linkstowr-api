use crate::error::{Error, Result};

#[derive(Clone, Debug)]
pub struct Ctx {
    user_id: String,
}

impl Ctx {
    pub fn new(user_id: String) -> Self {
        Self { user_id }
    }
}

impl Ctx {
    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    pub fn try_user_id_tuple(&self) -> Result<(&str, &str)> {
        let parts: Vec<&str> = self.user_id.split(":").collect();

        match parts.len() {
            2 => Ok((parts[0], parts[1])),
            _ => Err(Error::SplitUserIdFail),
        }
    }
}
