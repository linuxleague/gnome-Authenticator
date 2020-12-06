use crate::config;
use secret_service::{Collection, EncryptionType, SecretService, SsError};
use sha2::{Digest, Sha512};

pub struct Keyring;

impl Keyring {
    pub fn get_default_collection(ss: &SecretService) -> Result<Collection, SsError> {
        let collection = match ss.get_default_collection() {
            Err(SsError::NoResult) => ss.create_collection("default", "default"),
            e => e,
        }?;
        collection.unlock()?;

        Ok(collection)
    }

    pub fn store(label: &str, token: &str) -> Result<String, SsError> {
        let token = token.as_bytes();
        let ss = SecretService::new(EncryptionType::Dh)?;
        let col = Self::get_default_collection(&ss)?;

        let token_id = hex::encode(Sha512::digest(token));
        let attributes = vec![
            ("application", config::APP_ID),
            ("type", "token"),
            ("token_id", &token_id),
        ];
        col.create_item(label, attributes, token, true, "plain")?;
        Ok(token_id)
    }

    pub fn token(token_id: &str) -> Result<Option<String>, SsError> {
        let ss = SecretService::new(EncryptionType::Dh)?;
        let col = Self::get_default_collection(&ss)?;
        let items = col.search_items(vec![
            ("type", "token"),
            ("token_id", token_id),
            ("application", config::APP_ID),
        ])?;
        Ok(match items.get(0) {
            Some(e) => Some(String::from_utf8(e.get_secret()?).unwrap()),
            _ => None,
        })
    }

    pub fn remove_token(token_id: &str) -> Result<(), SsError> {
        let ss = SecretService::new(EncryptionType::Dh)?;
        let col = Self::get_default_collection(&ss)?;
        let items = col.search_items(vec![
            ("type", "token"),
            ("token_id", token_id),
            ("application", config::APP_ID),
        ])?;
        match items.get(0) {
            Some(e) => e.delete(),
            _ => Err(SsError::NoResult),
        }
    }

    pub fn has_set_password() -> Result<bool, SsError> {
        let ss = SecretService::new(EncryptionType::Dh)?;
        let col = Self::get_default_collection(&ss)?;
        match col.search_items(vec![("type", "password"), ("application", config::APP_ID)]) {
            Ok(items) => Ok(match items.get(0) {
                Some(_) => true,
                _ => false,
            }),
            _ => Ok(false),
        }
    }

    pub fn set_password(password: &str) -> Result<(), SsError> {
        let ss = SecretService::new(EncryptionType::Dh)?;
        let col = Self::get_default_collection(&ss)?;
        let attributes = vec![("application", config::APP_ID), ("type", "password")];
        col.create_item(
            "Authenticator password",
            attributes,
            password.as_bytes(),
            true,
            "plain",
        )?;
        Ok(())
    }

    pub fn reset_password() -> Result<(), SsError> {
        let ss = SecretService::new(EncryptionType::Dh)?;
        let col = Self::get_default_collection(&ss)?;
        let items =
            col.search_items(vec![("type", "password"), ("application", config::APP_ID)])?;

        match items.get(0) {
            Some(i) => i.delete(),
            None => Err(SsError::NoResult),
        }
    }

    pub fn is_current_password(password: &str) -> Result<bool, SsError> {
        let ss = SecretService::new(EncryptionType::Dh)?;
        let col = Self::get_default_collection(&ss)?;
        let items =
            col.search_items(vec![("type", "password"), ("application", config::APP_ID)])?;
        Ok(match items.get(0) {
            Some(i) => i.get_secret()? == password.as_bytes(),
            None => false,
        })
    }
}
