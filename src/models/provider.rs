use super::algorithm::{Algorithm, OTPMethod};
use crate::{
    models::{database, otp, Account, AccountsModel, FAVICONS_PATH},
    schema::providers,
};
use anyhow::Result;
use core::cmp::Ordering;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use glib::{clone, Cast, StaticType, ToValue};
use gtk::{gdk_pixbuf, gio, glib, prelude::*, subclass::prelude::*};
use std::{
    cell::{Cell, RefCell},
    str::FromStr,
    string::ToString,
    time::{SystemTime, UNIX_EPOCH},
};
use unicase::UniCase;
use url::Url;

#[derive(Debug)]
pub struct ProviderPatch {
    pub name: String,
    pub website: Option<String>,
    pub help_url: Option<String>,
    pub image_uri: Option<String>,
    pub period: i32,
    pub digits: i32,
    pub default_counter: i32,
    pub algorithm: String,
    pub method: String,
    pub is_backup_restore: bool,
}

#[derive(Insertable)]
#[table_name = "providers"]
struct NewProvider {
    pub name: String,
    pub website: Option<String>,
    pub help_url: Option<String>,
    pub image_uri: Option<String>,
    pub period: i32,
    pub digits: i32,
    pub default_counter: i32,
    pub algorithm: String,
    pub method: String,
}

#[derive(Identifiable, Queryable, Associations, Hash, PartialEq, Eq, Debug, Clone)]
#[table_name = "providers"]
pub struct DiProvider {
    pub id: i32,
    pub name: String,
    pub website: Option<String>,
    pub help_url: Option<String>,
    pub image_uri: Option<String>,
    pub period: i32,
    pub digits: i32,
    pub default_counter: i32,
    pub algorithm: String,
    pub method: String,
}
mod imp {
    use super::*;
    use glib::{ParamSpec, ParamSpecObject, ParamSpecString, ParamSpecUInt, Value};
    use gst::glib::{ParamSpecUInt64, SourceId};

    pub struct Provider {
        pub id: Cell<u32>,
        pub name: RefCell<String>,
        pub period: Cell<u32>,
        pub method: RefCell<String>,
        pub default_counter: Cell<u32>,
        pub algorithm: RefCell<String>,
        pub digits: Cell<u32>,
        pub website: RefCell<Option<String>>,
        pub help_url: RefCell<Option<String>>,
        pub image_uri: RefCell<Option<String>>,
        pub remaining_time: Cell<u64>,
        pub accounts: AccountsModel,
        pub filter_model: gtk::FilterListModel,
        pub tick_callback: RefCell<Option<SourceId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Provider {
        const NAME: &'static str = "Provider";
        type Type = super::Provider;

        fn new() -> Self {
            let model = AccountsModel::new();
            Self {
                id: Cell::new(0),
                default_counter: Cell::new(otp::HOTP_DEFAULT_COUNTER),
                algorithm: RefCell::new(Algorithm::default().to_string()),
                digits: Cell::new(otp::DEFAULT_DIGITS),
                name: RefCell::new("".to_string()),
                website: RefCell::new(None),
                help_url: RefCell::new(None),
                image_uri: RefCell::new(None),
                method: RefCell::new(OTPMethod::default().to_string()),
                period: Cell::new(otp::TOTP_DEFAULT_PERIOD),
                filter_model: gtk::FilterListModel::new(Some(&model), gtk::Filter::NONE),
                accounts: model,
                tick_callback: RefCell::default(),
                remaining_time: Cell::new(0),
            }
        }
    }

    impl ObjectImpl for Provider {
        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecUInt::new(
                        "id",
                        "id",
                        "Id",
                        0,
                        u32::MAX,
                        0,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpecString::new("name", "name", "Name", None, glib::ParamFlags::READWRITE),
                    ParamSpecObject::new(
                        "accounts",
                        "accounts",
                        "accounts",
                        AccountsModel::static_type(),
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpecUInt::new(
                        "period",
                        "period",
                        "Period",
                        0,
                        1000,
                        otp::TOTP_DEFAULT_PERIOD,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpecUInt::new(
                        "digits",
                        "digits",
                        "Digits",
                        0,
                        1000,
                        otp::DEFAULT_DIGITS,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpecUInt::new(
                        "default-counter",
                        "default_counter",
                        "default_counter",
                        0,
                        u32::MAX,
                        otp::HOTP_DEFAULT_COUNTER,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpecString::new(
                        "algorithm",
                        "algorithm",
                        "Algorithm",
                        Some(&Algorithm::default().to_string()),
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpecString::new(
                        "method",
                        "method",
                        "Method",
                        Some(&OTPMethod::default().to_string()),
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpecString::new(
                        "website",
                        "website",
                        "Website",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpecString::new(
                        "help-url",
                        "help url",
                        "Help URL",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpecString::new(
                        "image-uri",
                        "image uri",
                        "Image URI",
                        None,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    ParamSpecUInt64::new(
                        "remaining-time",
                        "remaining time",
                        "the remaining time",
                        0,
                        u64::MAX,
                        0,
                        glib::ParamFlags::READWRITE,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _obj: &Self::Type, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "id" => {
                    let id = value.get().unwrap();
                    self.id.replace(id);
                }
                "name" => {
                    let name = value.get().unwrap();
                    self.name.replace(name);
                }
                "period" => {
                    let period = value.get().unwrap();
                    self.period.replace(period);
                }
                "method" => {
                    let method = value.get().unwrap();
                    self.method.replace(method);
                }
                "digits" => {
                    let digits = value.get().unwrap();
                    self.digits.replace(digits);
                }
                "algorithm" => {
                    let algorithm = value.get().unwrap();
                    self.algorithm.replace(algorithm);
                }
                "default-counter" => {
                    let default_counter = value.get().unwrap();
                    self.default_counter.replace(default_counter);
                }
                "website" => {
                    let website = value.get().unwrap();
                    self.website.replace(website);
                }
                "help-url" => {
                    let help_url = value.get().unwrap();
                    self.help_url.replace(help_url);
                }
                "image-uri" => {
                    let image_uri = value.get().unwrap();
                    self.image_uri.replace(image_uri);
                }
                "remaining-time" => {
                    let remaining_time = value.get().unwrap();
                    self.remaining_time.set(remaining_time);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "id" => self.id.get().to_value(),
                "name" => self.name.borrow().to_value(),
                "period" => self.period.get().to_value(),
                "method" => self.method.borrow().to_value(),
                "digits" => self.digits.get().to_value(),
                "algorithm" => self.algorithm.borrow().to_value(),
                "default-counter" => self.default_counter.get().to_value(),
                "website" => self.website.borrow().to_value(),
                "help-url" => self.help_url.borrow().to_value(),
                "image-uri" => self.image_uri.borrow().to_value(),
                "accounts" => self.accounts.to_value(),
                "remaining-time" => self.remaining_time.get().to_value(),
                _ => unimplemented!(),
            }
        }

        fn dispose(&self, _obj: &Self::Type) {
            // Stop ticking
            if let Some(source_id) = self.tick_callback.borrow_mut().take() {
                source_id.remove();
            }
        }
    }
}

glib::wrapper! {
    pub struct Provider(ObjectSubclass<imp::Provider>);
}

impl Provider {
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        name: &str,
        period: u32,
        algorithm: Algorithm,
        website: Option<String>,
        method: OTPMethod,
        digits: u32,
        default_counter: u32,
        help_url: Option<String>,
        image_uri: Option<String>,
    ) -> Result<Self> {
        let db = database::connection();
        let conn = db.get()?;

        diesel::insert_into(providers::table)
            .values(NewProvider {
                name: name.to_string(),
                period: period as i32,
                method: method.to_string(),
                website,
                algorithm: algorithm.to_string(),
                digits: digits as i32,
                default_counter: default_counter as i32,
                help_url,
                image_uri,
            })
            .execute(&conn)?;

        providers::table
            .order(providers::columns::id.desc())
            .first::<DiProvider>(&conn)
            .map_err(From::from)
            .map(From::from)
    }

    pub fn compare(obj1: &glib::Object, obj2: &glib::Object) -> Ordering {
        let provider1 = obj1.downcast_ref::<Provider>().unwrap();
        let provider2 = obj2.downcast_ref::<Provider>().unwrap();

        UniCase::new(provider1.name()).cmp(&UniCase::new(provider2.name()))
    }

    pub fn load() -> Result<impl Iterator<Item = Self>> {
        use crate::schema::providers::dsl::*;
        let db = database::connection();
        let conn = db.get()?;

        let results = providers
            .load::<DiProvider>(&conn)?
            .into_iter()
            .map(From::from)
            .map(|p: Provider| {
                Account::load(&p).unwrap().for_each(|a| p.add_account(&a));
                p
            });
        Ok(results)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: u32,
        name: &str,
        period: u32,
        method: OTPMethod,
        algorithm: Algorithm,
        digits: u32,
        default_counter: u32,
        website: Option<String>,
        help_url: Option<String>,
        image_uri: Option<String>,
    ) -> Provider {
        glib::Object::new(&[
            ("id", &id),
            ("name", &name),
            ("website", &website),
            ("help-url", &help_url),
            ("image-uri", &image_uri),
            ("period", &period),
            ("method", &method.to_string()),
            ("algorithm", &algorithm.to_string()),
            ("digits", &digits),
            ("default-counter", &default_counter),
        ])
        .expect("Failed to create provider")
    }

    pub async fn favicon(
        website: String,
        name: String,
        id: u32,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let website_url = Url::parse(&website)?;
        let favicon = favicon_scrapper::Scrapper::from_url(website_url).await?;
        tracing::debug!("Found the following icons {:#?} for {}", favicon, name);

        let icon_name = format!("{}_{}", id, name.replace(' ', "_"));
        let icon_name = glib::base64_encode(icon_name.as_bytes());
        let small_icon_name = format!("{icon_name}_32x32");
        let large_icon_name = format!("{icon_name}_96x96");
        // TODO: figure out why trying to grab icons at specific size causes stack size errors
        // We need two sizes:
        // - 32x32 for the accounts lists
        // - 96x96 elsewhere
        if let Some(best_favicon) = favicon.find_best().await {
            tracing::debug!("Largest favicon found is {:#?}", best_favicon);
            let cache_path = FAVICONS_PATH.join(&*icon_name);
            best_favicon.save(cache_path.clone()).await?;
            // Don't try to scale down svg variants
            if !best_favicon.metadata().format().is_svg() {
                tracing::debug!("Creating scaled down variants for {:#?}", cache_path);
                {
                    let pixbuf = gdk_pixbuf::Pixbuf::from_file(cache_path.clone())?;
                    tracing::debug!("Creating a 32x32 variant of the favicon");
                    let small_pixbuf = pixbuf
                        .scale_simple(32, 32, gdk_pixbuf::InterpType::Bilinear)
                        .unwrap();

                    let mut small_cache = cache_path.clone();
                    small_cache.set_file_name(small_icon_name);
                    small_pixbuf.savev(small_cache.clone(), "png", &[])?;

                    tracing::debug!("Creating a 96x96 variant of the favicon");
                    let large_pixbuf = pixbuf
                        .scale_simple(96, 96, gdk_pixbuf::InterpType::Bilinear)
                        .unwrap();
                    let mut large_cache = cache_path.clone();
                    large_cache.set_file_name(large_icon_name);
                    large_pixbuf.savev(large_cache.clone(), "png", &[])?;
                };
                tokio::fs::remove_file(cache_path).await?;
            } else {
                let mut small_cache = cache_path.clone();
                small_cache.set_file_name(small_icon_name);
                tokio::fs::symlink(&cache_path, small_cache).await?;

                let mut large_cache = cache_path.clone();
                large_cache.set_file_name(large_icon_name);
                tokio::fs::symlink(&cache_path, large_cache).await?;
            }
            Ok(icon_name.to_string())
        } else {
            Err(Box::new(favicon_scrapper::Error::NoResults))
        }
    }

    pub fn id(&self) -> u32 {
        self.imp().id.get()
    }

    pub fn name(&self) -> String {
        self.imp().name.borrow().clone()
    }

    pub fn digits(&self) -> u32 {
        self.imp().digits.get()
    }

    pub fn default_counter(&self) -> u32 {
        self.imp().default_counter.get()
    }

    pub fn period(&self) -> u32 {
        self.imp().period.get()
    }

    pub fn algorithm(&self) -> Algorithm {
        Algorithm::from_str(&self.imp().algorithm.borrow().clone()).unwrap()
    }

    pub fn method(&self) -> OTPMethod {
        OTPMethod::from_str(&self.imp().method.borrow().clone()).unwrap()
    }

    pub fn website(&self) -> Option<String> {
        self.imp()
            .website
            .borrow()
            .clone()
            .and_then(|w| if w.is_empty() { None } else { Some(w) })
    }

    pub fn help_url(&self) -> Option<String> {
        self.imp()
            .help_url
            .borrow()
            .clone()
            .and_then(|h| if h.is_empty() { None } else { Some(h) })
    }

    pub fn delete(&self) -> Result<()> {
        let db = database::connection();
        let conn = db.get()?;
        diesel::delete(providers::table.filter(providers::columns::id.eq(self.id() as i32)))
            .execute(&conn)?;
        Ok(())
    }

    pub fn update(&self, patch: &ProviderPatch) -> Result<()> {
        // Can't implement PartialEq because of how GObject works
        if patch.name == self.name()
            && patch.website == self.website()
            && patch.help_url == self.help_url()
            && patch.image_uri == self.image_uri()
            && patch.period == self.period() as i32
            && patch.digits == self.digits() as i32
            && patch.default_counter == self.default_counter() as i32
            && patch.algorithm == self.algorithm().to_string()
            && patch.method == self.method().to_string()
        {
            return Ok(());
        }

        let db = database::connection();
        let conn = db.get()?;

        let target = providers::table.filter(providers::columns::id.eq(self.id() as i32));
        diesel::update(target)
            .set((
                providers::columns::algorithm.eq(&patch.algorithm),
                providers::columns::method.eq(&patch.method),
                providers::columns::digits.eq(&patch.digits),
                providers::columns::period.eq(&patch.period),
                providers::columns::default_counter.eq(&patch.default_counter),
                providers::columns::name.eq(&patch.name),
            ))
            .execute(&conn)?;
        if !patch.is_backup_restore {
            diesel::update(target)
                .set((
                    providers::columns::image_uri.eq(&patch.image_uri),
                    providers::columns::website.eq(&patch.website),
                    providers::columns::help_url.eq(&patch.help_url),
                ))
                .execute(&conn)?;
        };

        self.set_properties(&[
            ("name", &patch.name),
            ("period", &(patch.period as u32)),
            ("method", &patch.method),
            ("digits", &(patch.digits as u32)),
            ("algorithm", &patch.algorithm),
            ("default-counter", &(patch.default_counter as u32)),
        ]);

        if !patch.is_backup_restore {
            self.set_properties(&[
                ("image-uri", &patch.image_uri),
                ("website", &patch.website),
                ("help-url", &patch.help_url),
            ]);
        }
        Ok(())
    }

    pub fn set_image_uri(&self, uri: &str) -> Result<()> {
        let db = database::connection();
        let conn = db.get()?;

        let target = providers::table.filter(providers::columns::id.eq(self.id() as i32));
        diesel::update(target)
            .set(providers::columns::image_uri.eq(uri))
            .execute(&conn)?;

        self.set_property("image-uri", &uri);
        self.notify("image-uri");
        Ok(())
    }

    pub fn image_uri(&self) -> Option<String> {
        self.imp().image_uri.borrow().clone()
    }

    pub fn open_help(&self) {
        if let Some(ref url) = self.help_url() {
            gio::AppInfo::launch_default_for_uri(url, None::<&gio::AppLaunchContext>).unwrap();
        }
    }

    fn tick(&self) {
        let period = self.period() as u64;
        let remaining_time: u64 = period
            - SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                % period;
        if period == remaining_time {
            self.regenerate_otp();
        }
        self.set_property("remaining-time", &remaining_time);
    }

    fn setup_tick_callback(&self) {
        self.set_property("remaining-time", &(self.period() as u64));

        match self.method() {
            OTPMethod::TOTP | OTPMethod::Steam => {
                let source_id = glib::timeout_add_seconds_local(
                    1,
                    clone!(@weak self as provider => @default-return glib::Continue(false), move || {
                        provider.tick();
                        glib::Continue(true)
                    }),
                );
                self.imp().tick_callback.replace(Some(source_id));
            }
            _ => (),
        };
    }

    fn regenerate_otp(&self) {
        let accounts = self.accounts();
        for i in 0..accounts.n_items() {
            let item = accounts.item(i).unwrap();
            let account = item.downcast_ref::<Account>().unwrap();
            account.generate_otp();
        }
    }

    pub fn has_account(&self, account: &Account) -> Option<u32> {
        self.imp().accounts.find_position_by_id(account.id())
    }

    pub fn has_accounts(&self) -> bool {
        self.imp().accounts.n_items() != 0
    }

    pub fn add_account(&self, account: &Account) {
        self.imp().accounts.insert(account);
        if self.imp().tick_callback.borrow().is_none() && self.method().is_time_based() {
            self.setup_tick_callback();
        }
    }

    pub fn accounts_model(&self) -> &AccountsModel {
        &self.imp().accounts
    }

    fn tokenize_search(account_name: &str, provider_name: &str, term: &str) -> bool {
        let term = term.to_ascii_lowercase();
        let provider_name = provider_name.to_ascii_lowercase();
        let account_name = account_name.to_ascii_lowercase();

        account_name.split_ascii_whitespace().any(|x| x == term)
            || provider_name.split_ascii_whitespace().any(|x| x == term)
            || account_name.contains(term.as_str())
            || provider_name.contains(term.as_str())
    }

    pub fn find_accounts(&self, terms: &[String]) -> Vec<Account> {
        let mut results = vec![];
        let model = self.accounts_model();
        let provider_name = self.name();
        for pos in 0..model.n_items() {
            let obj = model.item(pos).unwrap();
            let account = obj.downcast::<Account>().unwrap();
            let account_name = account.name();

            if terms
                .iter()
                .any(|term| Self::tokenize_search(&account_name, &provider_name, term))
            {
                results.push(account);
            }
        }
        results
    }

    pub fn accounts(&self) -> &gtk::FilterListModel {
        &self.imp().filter_model
    }

    pub fn filter(&self, text: String) {
        let filter = gtk::CustomFilter::new(
            glib::clone!(@weak self as provider => @default-return false, move |obj| {
                let account = obj.downcast_ref::<Account>().unwrap();
                let account_name = account.name();
                let provider_name = provider.name();

                Self::tokenize_search(&account_name, &provider_name, &text)
            }),
        );
        self.imp().filter_model.set_filter(Some(&filter));
    }

    pub fn remove_account(&self, account: &Account) {
        let imp = self.imp();
        if let Some(pos) = imp.accounts.find_position_by_id(account.id()) {
            imp.accounts.remove(pos);
            if !self.has_accounts() && self.method().is_time_based() {
                // Stop ticking
                if let Some(source_id) = imp.tick_callback.borrow_mut().take() {
                    source_id.remove();
                }
            }
        }
    }
}

impl From<DiProvider> for Provider {
    fn from(p: DiProvider) -> Self {
        Self::new(
            p.id as u32,
            &p.name,
            p.period as u32,
            OTPMethod::from_str(&p.method).unwrap(),
            Algorithm::from_str(&p.algorithm).unwrap(),
            p.digits as u32,
            p.default_counter as u32,
            p.website,
            p.help_url,
            p.image_uri,
        )
    }
}

impl From<&Provider> for DiProvider {
    fn from(p: &Provider) -> Self {
        Self {
            id: p.id() as i32,
            name: p.name(),
            period: p.period() as i32,
            method: p.method().to_string(),
            algorithm: p.algorithm().to_string(),
            digits: p.digits() as i32,
            default_counter: p.default_counter() as i32,
            website: p.website(),
            help_url: p.help_url(),
            image_uri: p.image_uri(),
        }
    }
}
