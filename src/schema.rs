diesel::table! {
    users (id) {
        id -> Uuid,
        username -> Text,
        email -> Text,
        password_hash -> Text,
        bio -> Nullable<Text>,
        avatar_url -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        last_seen -> Nullable<Timestamp>,
        last_name -> Text,
        first_name -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    users,
);
