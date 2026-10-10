use crate::schema::*;
use diesel::prelude::*;
use serde::Serialize;

#[derive(Debug, Queryable, Selectable, Serialize)]
#[diesel(table_name = zsj)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Zsj {
    pub kod: String,
    pub nazev: String,
    pub lat: f64,
    pub lon: f64,
    pub kod_obce: Option<String>,
}

#[derive(Debug, Queryable, Selectable, Serialize)]
#[diesel(table_name = stredni_skoly)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Skola {
    pub redizo: String,
    pub nazev: Option<String>,
    pub kod_zsj: Option<String>,
    pub adresa: Option<String>,
    pub lat: Option<bigdecimal::BigDecimal>,
    pub lon: Option<bigdecimal::BigDecimal>,
    pub web: Option<String>,
}

#[derive(Debug, Queryable, Selectable, Serialize)]
#[diesel(table_name = obory)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Obor {
    pub kod: String,
    pub nazev: String,
}

#[derive(Debug, Queryable, Selectable, Serialize)]
#[diesel(table_name = nabidka_oboru)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NabidkaOboru {
    pub id: uuid::Uuid,
    pub redizo: String,
    pub kod_oboru: String,
    pub display_name: Option<String>,
    pub forma_studia: String,
    pub delka_studia: i32,
    pub pocet_prijimanych: i32,
    pub loni_pocet_prihlasek: i32,
    pub loni_pocet_prijatych: Option<i32>,
}

#[derive(Debug, Queryable, Selectable, Serialize)]
#[diesel(table_name = zamestnavatele)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Zamestnavatel {
    pub id: i64,
    pub ico: String,
    pub nazev: String,
    pub kod_adresniho_mista: Option<String>,
    pub kod_obce: String,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
}

#[derive(Debug, Queryable, Selectable, Serialize)]
#[diesel(table_name = profesni_skupiny)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ProfesniSkupina {
    pub cz_isco3: String,
    pub nazev: String,
}

#[derive(Debug, Queryable, Selectable, Serialize)]
#[diesel(table_name = poptavka_profesi)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PoptavkaProfesi {
    pub zamestnavatel_id: i64,
    pub cz_isco3: String,
    pub min_vzdelani: String,
    pub pocet_mist: i32,
    pub importovano_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Queryable, Selectable, Serialize)]
#[diesel(table_name = obor_profese)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct OborProfese {
    pub cz_isco3: String,
    pub kod_oboru: String,
    pub vhodnost: i16,
}

#[derive(Debug, Queryable, Selectable, Serialize)]
#[diesel(table_name = demo_skupina)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DemoSkupina {
    pub id: String,
    pub vek_od_do: String,
}

#[derive(Debug, Queryable, Selectable, Serialize)]
#[diesel(table_name = data_demografie_zsj)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DemografieZsj {
    pub kod_zsj: String,
    pub rok: i32,
    pub demo_skupina: String,
    pub populace: i32,
}

#[derive(Debug, Queryable, Selectable, Serialize)]
#[diesel(table_name = dojezdove_doby)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DojezdovaDoba {
    pub kod_zsj: String,
    pub redizo: String,
    pub slot_prijezdu: String,
    pub doba_jizdy: Option<f32>,
}
