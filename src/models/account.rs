use std::cmp::Ordering;

use anyhow::{Context, Result};
use diesel::prelude::*;
use gtk::{
    glib::{self, clone},
    prelude::*,
    subclass::prelude::*,
};
use unicase::UniCase;

use super::Token;
use crate::{
    models::{database, keyring, otp, DieselProvider, Method, OTPUri, Provider, RUNTIME},
    schema::accounts,
    utils::spawn_tokio_blocking,
    widgets::QRCodeData,
};

#[derive(Insertable)]
#[diesel(table_name = accounts)]
struct NewAccount {
    pub name: String,
    pub token_id: String,
    pub provider_id: i32,
    pub counter: i32,
}

#[derive(Identifiable, Queryable, Associations)]
#[diesel(belongs_to(DieselProvider, foreign_key = provider_id))]
#[diesel(table_name = accounts)]
pub struct DieselAccount {
    pub id: i32,
    pub name: String,
    pub counter: i32,
    pub token_id: String,
    pub provider_id: i32,
}

#[doc(hidden)]
mod imp {
    use std::cell::{Cell, RefCell};

    use glib::ParamSpecObject;
    use once_cell::sync::{Lazy, OnceCell};

    use super::*;
    use crate::models::Token;

    #[derive(glib::Properties)]
    #[properties(wrapper_type = super::Account)]
    pub struct Account {
        #[property(get, set, construct)]
        pub id: Cell<u32>,
        #[property(get, set)]
        pub otp: RefCell<String>,
        #[property(get, set = Self::set_name)]
        pub name: RefCell<String>,
        #[property(get, set = Self::set_counter, default = otp::HOTP_DEFAULT_COUNTER)]
        pub counter: Cell<u32>,
        pub token: OnceCell<Token>,
        #[property(get, set, construct_only)]
        pub token_id: RefCell<String>,
        // We don't use property here as we can't mark the getter as not nullable
        pub provider: RefCell<Option<Provider>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Account {
        const NAME: &'static str = "Account";
        type Type = super::Account;

        fn new() -> Self {
            Self {
                id: Cell::default(),
                counter: Cell::new(otp::HOTP_DEFAULT_COUNTER),
                name: RefCell::default(),
                otp: RefCell::default(),
                token_id: RefCell::default(),
                provider: RefCell::default(),
                token: OnceCell::default(),
            }
        }
    }

    impl ObjectImpl for Account {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                let mut props = Account::derived_properties().to_vec();
                props.push(ParamSpecObject::builder::<Provider>("provider").build());
                props
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "provider" => {
                    let provider = value.get().unwrap();
                    self.provider.replace(provider);
                }
                _ => self.derived_set_property(id, value, pspec),
            }
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "provider" => self.provider.borrow().to_value(),
                _ => self.derived_property(id, pspec),
            }
        }
    }

    impl Account {
        fn set_name_inner(&self, id: i32, name: &str) -> Result<()> {
            let db = database::connection();
            let mut conn = db.get()?;

            let target = accounts::table.filter(accounts::columns::id.eq(id));
            diesel::update(target)
                .set(accounts::columns::name.eq(name))
                .execute(&mut conn)?;
            Ok(())
        }

        fn set_name(&self, name: &str) {
            match self.set_name_inner(self.obj().id() as i32, name) {
                Ok(_) => {
                    self.name.replace(name.to_owned());
                }
                Err(err) => {
                    tracing::warn!("Failed to update account name {err}");
                }
            }
        }

        fn set_counter_inner(&self, id: i32, counter: u32) -> Result<()> {
            let db = database::connection();
            let mut conn = db.get()?;

            let target = accounts::table.filter(accounts::columns::id.eq(id));
            diesel::update(target)
                .set(accounts::columns::counter.eq(counter as i32))
                .execute(&mut conn)?;
            Ok(())
        }

        fn set_counter(&self, counter: u32) {
            match self.set_counter_inner(self.obj().id() as i32, counter) {
                Ok(_) => {
                    self.counter.set(counter);
                }
                Err(err) => {
                    tracing::warn!("Failed to update account counter {err}");
                }
            }
        }
    }
}

glib::wrapper! {
    pub struct Account(ObjectSubclass<imp::Account>);
}

impl Account {
    pub fn create(
        name: &str,
        token: &str,
        counter: Option<u32>,
        provider: &Provider,
    ) -> Result<Account> {
        let db = database::connection();
        let mut conn = db.get()?;

        let label = format!("{} - {name}", provider.name());
        let token_send = token.to_owned();
        let token_id = spawn_tokio_blocking(async move {
            keyring::store(&label, &token_send)
                .await
                .context("Failed to save token")
        })?;

        diesel::insert_into(accounts::table)
            .values(NewAccount {
                name: name.to_string(),
                token_id,
                provider_id: provider.id() as i32,
                counter: counter.unwrap_or_else(|| provider.default_counter()) as i32,
            })
            .execute(&mut conn)?;

        accounts::table
            .order(accounts::columns::id.desc())
            .first::<DieselAccount>(&mut conn)
            .map_err(From::from)
            .map(|account| {
                Self::new(
                    account.id as u32,
                    &account.name,
                    &account.token_id,
                    account.counter as u32,
                    provider.clone(),
                    Some(token),
                )
                .unwrap()
            })
    }

    pub fn load(p: &Provider) -> Result<impl Iterator<Item = Self>> {
        let db = database::connection();
        let mut conn = db.get()?;

        let dip = DieselProvider::from(p);
        let results = DieselAccount::belonging_to(&dip)
            .load::<DieselAccount>(&mut conn)?
            .into_iter()
            .filter_map(clone!(@strong p => move |account| {
                match Self::new(
                    account.id  as u32,
                    &account.name,
                    &account.token_id,
                    account.counter as u32,
                    p.clone(),
                    None,
                )
                {
                    Ok(account) => Some(account),
                    Err(e) => {
                        let name = account.name;
                        let provider = p.name();
                        tracing::error!("Failed to load account '{name}' / '{provider}' with error {e}");
                        None
                    }
                }
            }));

        Ok(results)
    }

    pub fn compare(obj1: &glib::Object, obj2: &glib::Object) -> Ordering {
        let account1 = obj1.downcast_ref::<Account>().unwrap();
        let account2 = obj2.downcast_ref::<Account>().unwrap();

        UniCase::new(account1.name()).cmp(&UniCase::new(account2.name()))
    }

    pub fn new(
        id: u32,
        name: &str,
        token_id: &str,
        counter: u32,
        provider: Provider,
        token: Option<&str>,
    ) -> Result<Account> {
        let account = glib::Object::builder::<Self>()
            .property("id", id)
            .property("name", name)
            .property("token-id", token_id)
            .property("provider", provider.clone())
            .property("counter", counter)
            .build();

        let token = if let Some(t) = token {
            t.to_string()
        } else {
            let token_id = token_id.to_owned();
            spawn_tokio_blocking(async move {
                keyring::token(&token_id).await?.with_context(|| {
                    format!("Could not get item with token identifier '{token_id}' from keyring")
                })
            })?
        };
        let token = Token::from_str(&token, provider.algorithm(), provider.digits())?;
        account.imp().token.set(token).unwrap();
        account.generate_otp();
        Ok(account)
    }

    pub fn generate_otp(&self) {
        let provider = self.provider();

        let counter = match provider.method() {
            Method::TOTP => otp::time_based_counter(provider.period()),
            Method::HOTP => self.counter() as u64,
            Method::Steam => otp::time_based_counter(otp::STEAM_DEFAULT_PERIOD),
        };

        let otp_password: Result<String> = match provider.method() {
            Method::Steam => self.token().steam(counter),
            _ => self.token().hotp_formatted(counter),
        };

        let label = match otp_password {
            Ok(password) => password,
            Err(err) => {
                tracing::warn!("Failed to generate the OTP {}", err);
                "Error".to_string()
            }
        };

        self.set_otp(label);
    }

    /// Increment the internal counter in case of a HOTP account
    pub fn increment_counter(&self) -> Result<()> {
        let new_value = self.counter() + 1;
        self.imp().counter.set(new_value);

        let db = database::connection();
        let mut conn = db.get()?;

        let target = accounts::table.filter(accounts::columns::id.eq(self.id() as i32));
        diesel::update(target)
            .set(accounts::columns::counter.eq(new_value as i32))
            .execute(&mut conn)?;
        Ok(())
    }

    pub fn copy_otp(&self) {
        let display = gtk::gdk::Display::default().unwrap();
        let clipboard = display.clipboard();
        // The codes come with the white space shown in the label.
        let code = &self.imp().otp.borrow().replace(' ', "");
        clipboard.set_text(code);

        // Indirectly increment the counter once the token was copied
        if self.provider().method().is_event_based() {
            self.generate_otp();
        }
    }

    pub fn provider(&self) -> Provider {
        self.property("provider")
    }

    pub fn set_provider(&self, provider: &Provider) -> Result<()> {
        let db = database::connection();
        let mut conn = db.get()?;

        let target = accounts::table.filter(accounts::columns::id.eq(self.id() as i32));
        diesel::update(target)
            .set(accounts::columns::provider_id.eq(provider.id() as i32))
            .execute(&mut conn)?;
        self.set_property("provider", provider);
        Ok(())
    }

    pub fn token(&self) -> &Token {
        self.imp().token.get().unwrap()
    }

    pub fn otp_uri(&self) -> OTPUri {
        self.into()
    }

    pub fn qr_code(&self) -> QRCodeData {
        let otp: String = self.otp_uri().into();
        QRCodeData::from(otp.as_str())
    }

    pub fn delete(&self) -> Result<()> {
        let token_id = self.token_id();
        RUNTIME.spawn(async move {
            if let Err(err) = keyring::remove_token(&token_id).await {
                tracing::error!("Failed to remove the token from secret service {}", err);
            }
        });
        let db = database::connection();
        let mut conn = db.get()?;
        diesel::delete(accounts::table.filter(accounts::columns::id.eq(self.id() as i32)))
            .execute(&mut conn)?;
        Ok(())
    }
}
