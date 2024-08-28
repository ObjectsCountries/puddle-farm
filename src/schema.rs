// @generated automatically by Diesel CLI.

diesel::table! {
    games (timestamp, id_a, id_b) {
        timestamp -> Timestamp,
        id_a -> Int8,
        name_a -> Text,
        char_a -> Int2,
        platform_a -> Int2,
        id_b -> Int8,
        name_b -> Text,
        char_b -> Int2,
        platform_b -> Int2,
        winner -> Int2,
        game_floor -> Int2,
    }
}

diesel::table! {
    player_names (id, name) {
        id -> Int8,
        name -> Text,
    }
}

diesel::table! {
    player_ratings (id, char_id) {
        id -> Int8,
        char_id -> Int2,
        wins -> Int4,
        losses -> Int4,
        value -> Float4,
        deviation -> Float4,
        last_decay -> Timestamp,
        top_rating_value -> Nullable<Float4>,
        top_rating_deviation -> Nullable<Float4>,
        top_rating_timestamp -> Nullable<Timestamp>,
        top_defeated_id -> Nullable<Int8>,
        top_defeated_char_id -> Nullable<Int2>,
        top_defeated_name -> Nullable<Text>,
        top_defeated_value -> Nullable<Float4>,
        top_defeated_deviation -> Nullable<Float4>,
        top_defeated_timestamp -> Nullable<Timestamp>,
    }
}

diesel::table! {
    players (id) {
        id -> Int8,
        name -> Text,
        platform -> Int2,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    games,
    player_names,
    player_ratings,
    players,
);
