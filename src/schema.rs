table! {
    accounts (id) {
        id -> Integer,
        name -> Text,
        counter -> Integer,
        token_id -> Text,
        provider_id -> Integer,
    }
}

table! {
    providers (id) {
        id -> Integer,
        name -> Text,
        website -> Nullable<Text>,
        help_url -> Nullable<Text>,
        image_uri -> Nullable<Text>,
        period -> Integer,
        digits -> Integer,
        default_counter -> Integer,
        algorithm -> Text,
        method -> Text,
    }
}

joinable!(accounts -> providers (provider_id));
allow_tables_to_appear_in_same_query!(accounts, providers);
