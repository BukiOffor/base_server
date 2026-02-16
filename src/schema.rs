// @generated automatically by Diesel CLI.

diesel::table! {
    organisations (id) {
        id -> Uuid,
        name -> Text,
        description -> Nullable<Text>,
        logo_url -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    tour_links (id) {
        id -> Uuid,
        source_node_id -> Uuid,
        target_node_id -> Uuid,
        yaw -> Float8,
        pitch -> Float8,
        label -> Nullable<Text>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    tours (id) {
        id -> Uuid,
        organisation_id -> Uuid,
        name -> Text,
        description -> Nullable<Text>,
        panorama_url -> Text,
        created_by -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        username -> Text,
        email -> Text,
        password_hash -> Text,
        first_name -> Text,
        last_name -> Text,
        bio -> Nullable<Text>,
        avatar_url -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        last_seen -> Nullable<Timestamp>,
        role -> Text,
        organisation_id -> Nullable<Uuid>,
    }
}

diesel::joinable!(tours -> organisations (organisation_id));
diesel::joinable!(tours -> users (created_by));
diesel::joinable!(users -> organisations (organisation_id));

diesel::allow_tables_to_appear_in_same_query!(
    organisations,
    tour_links,
    tours,
    users,
);
