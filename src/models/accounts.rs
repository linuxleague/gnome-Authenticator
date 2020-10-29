use super::account::Account;
use super::provider::Provider;
use gio::prelude::*;
use std::cell::RefCell;

#[derive(Clone, PartialEq)]
pub enum SortOrder {
    Asc,
    Desc,
}

#[derive(Clone, PartialEq)]
pub enum SortBy {
    Name,
    Date,
}

impl From<&str> for SortBy {
    fn from(sortby: &str) -> Self {
        match sortby {
            "name" => SortBy::Name,
            "date" => SortBy::Date,
            _ => SortBy::Name,
        }
    }
}

pub struct AccountsModel {
    pub model: gio::ListStore,
    sort_order: RefCell<SortOrder>,
    sort_by: RefCell<SortBy>,
    provider: Provider,
}

impl AccountsModel {
    pub fn from_provider(provider: &Provider) -> Self {
        let gio_model = gio::ListStore::new(Account::static_type());
        let model = Self {
            model: gio_model,
            sort_order: RefCell::new(SortOrder::Desc),
            sort_by: RefCell::new(SortBy::Name),
            provider: provider.clone(),
        };
        model.init();
        model
    }

    fn init(&self) {
        // fill in the accounts from the database
        /*let accounts = database::get_accounts_by_provider(self.provider.clone()).unwrap();

        for account in accounts.into_iter() {
            self.add_account(&account);
        }*/
    }

    fn add_account(&self, account: &Account) {
        let sort_by = self.sort_by.clone();
        let sort_order = self.sort_order.clone();
        self.model.insert_sorted(account, move |a, b| {
            Self::accounts_cmp(a, b, sort_by.borrow().clone(), sort_order.borrow().clone())
        });
    }

    pub fn get_count(&self) -> u32 {
        self.model.get_n_items()
    }

    pub fn set_sorting(&self, sort_by: Option<SortBy>, sort_order: Option<SortOrder>) {
        let sort_by = match sort_by {
            Some(sort_by) => {
                self.sort_by.replace(sort_by.clone());
                sort_by
            }
            None => self.sort_by.borrow().clone(),
        };

        let sort_order = match sort_order {
            Some(sort_order) => {
                self.sort_order.replace(sort_order.clone());
                sort_order
            }
            None => self.sort_order.borrow().clone(),
        };
        self.model
            .sort(move |a, b| Self::accounts_cmp(a, b, sort_by.clone(), sort_order.clone()));
    }

    fn accounts_cmp(
        a: &glib::Object,
        b: &glib::Object,
        sort_by: SortBy,
        sort_order: SortOrder,
    ) -> std::cmp::Ordering {
        let mut account_a: &Account = a.downcast_ref::<Account>().unwrap();
        let mut account_b: &Account = b.downcast_ref::<Account>().unwrap();

        if sort_order == SortOrder::Desc {
            let tmp = account_a;
            account_a = account_b;
            account_b = tmp;
        }
        /*match sort_by {
            SortBy::Name => account_a.get_title().cmp(&account_b.get_title()),
            SortBy::Date => account_a.get_created_at().cmp(&account_b.get_created_at()),
        }*/
        account_a.name().cmp(&account_b.name())
    }
}
