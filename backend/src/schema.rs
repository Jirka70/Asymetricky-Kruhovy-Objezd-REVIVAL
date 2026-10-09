// SQL identifiers retain the case used by the source scripts.
pub mod sql_types {
    #[derive(diesel::sql_types::SqlType, diesel::query_builder::QueryId)]
    #[diesel(postgres_type(name = "geometry"))]
    pub struct Geometry;
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Geometry;
    #[sql_name = "ZSJ"]
    zsj (kod) {
        kod -> Text,
        nazev -> Text,
        lat -> Float8,
        lon -> Float8,
        boundary -> Geometry,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    #[sql_name = "stredni_skoly"]
    stredni_skoly (redizo) {
        redizo -> Varchar,
        nazev -> Nullable<Varchar>,
        kod_zsj -> Nullable<Varchar>,
        adresa -> Nullable<Varchar>,
        lat -> Nullable<Numeric>,
        lon -> Nullable<Numeric>,
        web -> Nullable<Varchar>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    #[sql_name = "OBORY"]
    obory (kod) {
        kod -> Text,
        nazev -> Text,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    #[sql_name = "NABIDKA_OBORU"]
    nabidka_oboru (id) {
        id -> Uuid,
        redizo -> Text,
        kod_oboru -> Text,
        display_name -> Nullable<Text>,
        forma_studia -> Text,
        delka_studia -> Int4,
        pocet_prijimanych -> Int4,
        loni_pocet_prihlasek -> Int4,
        loni_pocet_prijatych -> Nullable<Int4>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    #[sql_name = "ZAMESTNAVATELE"]
    zamestnavatele (id) {
        id -> Int8,
        ico -> Text,
        nazev -> Text,
        kod_adresniho_mista -> Nullable<Text>,
        kod_obce -> Text,
        lat -> Nullable<Float8>,
        lon -> Nullable<Float8>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    #[sql_name = "PROFESNI_SKUPINY"]
    profesni_skupiny (cz_isco3) {
        cz_isco3 -> Text,
        nazev -> Text,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    #[sql_name = "POPTAVKA_PROFESI"]
    poptavka_profesi (zamestnavatel_id, cz_isco3, min_vzdelani) {
        zamestnavatel_id -> Int8,
        cz_isco3 -> Text,
        min_vzdelani -> Text,
        pocet_mist -> Int4,
        importovano_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    #[sql_name = "OBOR_PROFESE"]
    obor_profese (cz_isco3, kod_oboru) {
        cz_isco3 -> Text,
        kod_oboru -> Text,
        vhodnost -> Int2,
    }
}

diesel::joinable!(nabidka_oboru -> obory (kod_oboru));
diesel::joinable!(poptavka_profesi -> zamestnavatele (zamestnavatel_id));
diesel::joinable!(poptavka_profesi -> profesni_skupiny (cz_isco3));
diesel::joinable!(obor_profese -> profesni_skupiny (cz_isco3));
diesel::joinable!(obor_profese -> obory (kod_oboru));
diesel::allow_tables_to_appear_in_same_query!(
    zsj,
    stredni_skoly,
    obory,
    nabidka_oboru,
    zamestnavatele,
    profesni_skupiny,
    poptavka_profesi,
    obor_profese
);
