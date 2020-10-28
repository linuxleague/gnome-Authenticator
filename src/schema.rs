table! {
    accounts (id) {
        id -> Integer,
        username -> Text,
        token_id -> Text,
        provider -> Integer,
    }
}

table! {
    providers (id) {
        id -> Integer,
        name -> Text,
        period -> Integer,
        algorithm -> Text,
        website -> Nullable<Text>,
        help_url -> Nullable<Text>,
        image_uri -> Nullable<Text>,
    }
}
