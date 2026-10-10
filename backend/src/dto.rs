//! Public API DTOs, separate from Diesel database models.
//! Contract tests validate their serialized values against the root openapi.yaml.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

fn deserialize_required_nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

pub type KodZsj = String;

pub type KodObce = String;

pub type KodPlochy = String;

pub type Redizo = String;

pub type KodOboru = String;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum Stupen {
    #[serde(rename = "C")]
    C,
    #[serde(rename = "E")]
    E,
    #[serde(rename = "H")]
    H,
    #[serde(rename = "K")]
    K,
    #[serde(rename = "L")]
    L,
    #[serde(rename = "M")]
    M,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum Signal {
    #[serde(rename = "pretlak")]
    Pretlak,
    #[serde(rename = "nizky_zajem")]
    NizkyZajem,
    #[serde(rename = "spatna_dostupnost")]
    SpatnaDostupnost,
    #[serde(rename = "poptavka_trhu")]
    PoptavkaTrhu,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum Pasmo {
    #[serde(rename = "do30")]
    Do30,
    #[serde(rename = "30_45")]
    Od30Do45,
    #[serde(rename = "45_60")]
    Od45Do60,
    #[serde(rename = "nad60")]
    Nad60,
    #[serde(rename = "mimo_dosah")]
    MimoDosah,
    #[serde(rename = "bez_spojeni")]
    BezSpojeni,
}

pub type Cas = String;

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ChybaError {
    pub kod: String,
    pub zprava: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pole: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Chyba {
    pub error: ChybaError,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct MetaDosah {
    pub scenar: String,
    pub max_min: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct MetaBilancePrijimaciRizeni {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rok: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kolo: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct MetaBilancePrahy {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pretlak: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nizky_zajem: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spatna_dostupnost: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poptavka_trhu: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct MetaBilance {
    pub scenar: String,
    pub max_min: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deti_celkem: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prumer_prihlasek_na_misto: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prijimaci_rizeni: Option<MetaBilancePrijimaciRizeni>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prahy: Option<MetaBilancePrahy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct VLimitu {
    pub limit_min: i64,
    pub jednotek: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct VLimituPredPo {
    pub limit_min: i64,
    pub pred: i64,
    pub po: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum NabidkaForma {
    #[serde(rename = "den")]
    Den,
    #[serde(rename = "dal")]
    Dal,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Nabidka {
    pub kod_oboru: KodOboru,
    pub nazev_oboru: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zamereni: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stupen: Option<Stupen>,
    pub forma: NabidkaForma,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delka_let: Option<i64>,
    pub kapacita: i64,
    pub prihlasky: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prihlasky_na_misto: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_pretlaku: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum GeoPointType {
    #[serde(rename = "Point")]
    Point,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct GeoPoint {
    pub r#type: GeoPointType,
    pub coordinates: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum GeoPlochaType {
    #[serde(rename = "Polygon")]
    Polygon,
    #[serde(rename = "MultiPolygon")]
    MultiPolygon,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct GeoPlocha {
    pub r#type: GeoPlochaType,
    pub coordinates: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum ZsjBoundaryType {
    #[serde(rename = "Polygon")]
    Polygon,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ZsjBoundary {
    pub r#type: ZsjBoundaryType,
    pub coordinates: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ZsjZaznam {
    pub kod: KodZsj,
    pub nazev: String,
    pub lat: f64,
    pub lon: f64,
    pub boundary: ZsjBoundary,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    #[schemars(required)]
    pub kod_obce: Option<KodObce>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum GeoLinieType {
    #[serde(rename = "LineString")]
    LineString,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct GeoLinie {
    pub r#type: GeoLinieType,
    pub coordinates: Vec<Vec<f64>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum SkolyFeatureCollectionType {
    #[serde(rename = "FeatureCollection")]
    FeatureCollection,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum SkolyFeatureCollectionFeaturesItemType {
    #[serde(rename = "Feature")]
    Feature,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SkolyFeatureCollectionFeaturesItemProperties {
    pub redizo: Redizo,
    pub nazev: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub obec: Option<String>,
    pub pocet_nabidek: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kapacita: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prihlasky: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_pretlaku: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SkolyFeatureCollectionFeaturesItem {
    pub r#type: SkolyFeatureCollectionFeaturesItemType,
    pub geometry: GeoPoint,
    pub properties: SkolyFeatureCollectionFeaturesItemProperties,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SkolyFeatureCollection {
    pub r#type: SkolyFeatureCollectionType,
    pub features: Vec<SkolyFeatureCollectionFeaturesItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SkolaDetailSpadovostObceItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kod_obce: Option<KodObce>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nazev: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deti: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nejkratsi_cas_min: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SkolaDetailSpadovost {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deti_v_dosahu: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub obce: Option<Vec<SkolaDetailSpadovostObceItem>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SkolaDetail {
    pub redizo: Redizo,
    pub nazev: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adresa: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub web: Option<String>,
    pub lat: f64,
    pub lon: f64,
    pub nabidky: Vec<Nabidka>,
    pub spadovost: SkolaDetailSpadovost,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaDosah>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum SpojStav {
    #[serde(rename = "ok")]
    Ok,
    #[serde(rename = "bez_spojeni")]
    BezSpojeni,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Spoj {
    pub stav: SpojStav,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub odjezd: Option<Cas>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prijezd: Option<Cas>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cas_min: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prestupy: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chuze_m: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linky: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct StudentSkolyDataItem {
    pub redizo: Redizo,
    pub nazev: String,
    pub lat: f64,
    pub lon: f64,
    pub v_dosahu: bool,
    pub spoj: Spoj,
    pub nabidky: Vec<Nabidka>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct StudentSkolyMeta {
    pub scenar: String,
    pub max_min: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zsj: Option<KodZsj>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub v_dosahu: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mimo_dosah: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presnost: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct StudentSkoly {
    pub data: Vec<StudentSkolyDataItem>,
    pub meta: StudentSkolyMeta,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum TrasaType {
    #[serde(rename = "FeatureCollection")]
    FeatureCollection,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum TrasaFeaturesItemType {
    #[serde(rename = "Feature")]
    Feature,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum TrasaFeaturesItemPropertiesDruh {
    #[serde(rename = "WALK")]
    WALK,
    #[serde(rename = "BUS")]
    BUS,
    #[serde(rename = "RAIL")]
    RAIL,
    #[serde(rename = "TRAM")]
    TRAM,
    #[serde(rename = "TROLLEYBUS")]
    TROLLEYBUS,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TrasaFeaturesItemProperties {
    pub spoj: i64,
    pub usek: i64,
    pub druh: TrasaFeaturesItemPropertiesDruh,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linka: Option<String>,
    pub od: String,
    pub r#do: String,
    pub odjezd: Cas,
    pub prijezd: Cas,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chuze_m: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TrasaFeaturesItem {
    pub r#type: TrasaFeaturesItemType,
    pub geometry: GeoLinie,
    pub properties: TrasaFeaturesItemProperties,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TrasaSpojeItem {
    pub spoj: i64,
    pub odjezd: Cas,
    pub prijezd: Cas,
    pub cas_min: i64,
    pub prestupy: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chuze_m: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rezerva_min: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TrasaMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenar: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub okno_prijezdu: Option<Vec<Cas>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub den: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Trasa {
    pub r#type: TrasaType,
    pub features: Vec<TrasaFeaturesItem>,
    pub spoje: Vec<TrasaSpojeItem>,
    pub meta: TrasaMeta,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum PlochaDosahuUroven {
    #[serde(rename = "orp")]
    Orp,
    #[serde(rename = "obec")]
    Obec,
    #[serde(rename = "zsj")]
    Zsj,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PlochaDosahu {
    pub kod: KodPlochy,
    pub nazev: String,
    pub uroven: PlochaDosahuUroven,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cas_min: Option<i64>,
    pub pasmo: Pasmo,
    pub v_dosahu: bool,
    pub deti: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deti_v_dosahu: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub podil_deti_v_dosahu: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nejblizsi_redizo: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum MapaDosahuType {
    #[serde(rename = "FeatureCollection")]
    FeatureCollection,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum MapaDosahuFeaturesItemType {
    #[serde(rename = "Feature")]
    Feature,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct MapaDosahuFeaturesItem {
    pub r#type: MapaDosahuFeaturesItemType,
    pub geometry: GeoPlocha,
    pub properties: PlochaDosahu,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum MapaDosahuMetaUroven {
    #[serde(rename = "orp")]
    Orp,
    #[serde(rename = "obec")]
    Obec,
    #[serde(rename = "zsj")]
    Zsj,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct MapaDosahuMeta {
    pub scenar: String,
    pub max_min: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub obor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forma: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uroven: Option<MapaDosahuMetaUroven>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jednotek_celkem: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub v_limitu: Option<Vec<VLimitu>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct MapaDosahu {
    pub r#type: MapaDosahuType,
    pub features: Vec<MapaDosahuFeaturesItem>,
    pub meta: MapaDosahuMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct BilanceOboru {
    pub kod: KodOboru,
    pub nazev: String,
    pub stupen: Stupen,
    pub pocet_skol: i64,
    pub kapacita: i64,
    pub prihlasky: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prihlasky_na_misto: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_pretlaku: Option<f64>,
    pub deti_v_dosahu: i64,
    pub deti_bez_oboru: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub podil_deti_v_dosahu: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mist_na_100_deti: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zamestnavatelu: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volna_mista: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volna_mista_na_misto: Option<f64>,
    pub signaly: Vec<Signal>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum DetailOboruNabidkyItemForma {
    #[serde(rename = "den")]
    Den,
    #[serde(rename = "dal")]
    Dal,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct DetailOboruNabidkyItem {
    pub kod_oboru: KodOboru,
    pub nazev_oboru: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zamereni: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stupen: Option<Stupen>,
    pub forma: DetailOboruNabidkyItemForma,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delka_let: Option<i64>,
    pub kapacita: i64,
    pub prihlasky: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prihlasky_na_misto: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_pretlaku: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redizo: Option<Redizo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nazev_skoly: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deti_v_dosahu_skoly: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct DetailOboruKandidatiItem {
    pub redizo: Redizo,
    pub nazev: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub obec: Option<String>,
    pub nove_dosazene_deti: i64,
    pub ma_pribuzny_obor: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct DetailOboruTrhPraceProfeseItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cz_isco3: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vhodnost: Option<crate::types::Vhodnost>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nazev: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pocet: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct DetailOboruTrhPrace {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volna_mista: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zamestnavatelu: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profese: Option<Vec<DetailOboruTrhPraceProfeseItem>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct DetailOboru {
    pub obor: BilanceOboru,
    pub nabidky: Vec<DetailOboruNabidkyItem>,
    pub kandidati: Vec<DetailOboruKandidatiItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trh_prace: Option<DetailOboruTrhPrace>,
    pub meta: MetaDosah,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum ZamestnavateleOboruType {
    #[serde(rename = "FeatureCollection")]
    FeatureCollection,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum ZamestnavateleOboruFeaturesItemType {
    #[serde(rename = "Feature")]
    Feature,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ZamestnavateleOboruFeaturesItemPropertiesProfeseItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cz_isco3: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nazev: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vhodnost: Option<crate::types::Vhodnost>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pocet_mist: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ZamestnavateleOboruFeaturesItemProperties {
    pub id: String,
    pub ico: String,
    pub nazev: String,
    pub kod_obce: KodObce,
    pub pocet_mist: i64,
    pub profese: Vec<ZamestnavateleOboruFeaturesItemPropertiesProfeseItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ZamestnavateleOboruFeaturesItem {
    pub r#type: ZamestnavateleOboruFeaturesItemType,
    pub geometry: GeoPoint,
    pub properties: ZamestnavateleOboruFeaturesItemProperties,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ZamestnavateleOboruMetaBezSouradnicItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kod_obce: Option<KodObce>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pracovist: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pocet_mist: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ZamestnavateleOboruMetaSkupinyItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cz_isco3: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nazev: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vhodnost: Option<crate::types::Vhodnost>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ZamestnavateleOboruMeta {
    pub obor: KodOboru,
    pub mapovani: bool,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    #[schemars(required)]
    pub existuje: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zamestnavatelu: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pracovist: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pocet_mist: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bez_souradnic: Option<Vec<ZamestnavateleOboruMetaBezSouradnicItem>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skupiny: Option<Vec<ZamestnavateleOboruMetaSkupinyItem>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub importovano: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ZamestnavateleOboru {
    pub r#type: ZamestnavateleOboruType,
    pub features: Vec<ZamestnavateleOboruFeaturesItem>,
    pub meta: ZamestnavateleOboruMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SimulacePlocha {
    pub nazev: String,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    #[schemars(required)]
    pub cas_ke_skole: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cas_min_puvodni: Option<i64>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    #[schemars(required)]
    pub cas_min: Option<i64>,
    pub zlepseni_min: i64,
    pub pasmo: Pasmo,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pasmo_puvodni: Option<Pasmo>,
    pub deti: i64,
    pub potencialni_uchazeci: f64,
    pub novy_dosah: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SimulaceSouhrn {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jednotek_celkem: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub v_limitu: Option<Vec<VLimituPredPo>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zlepsenych_jednotek: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prumerne_zkraceni_min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub potencialni_uchazeci: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blize_ke_skole: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kapacita: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uchazecu_na_misto: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum SimulaceMetaUroven {
    #[serde(rename = "orp")]
    Orp,
    #[serde(rename = "obec")]
    Obec,
    #[serde(rename = "zsj")]
    Zsj,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SimulaceMeta {
    pub scenar: String,
    pub max_min: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redizo: Option<Redizo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub obor: Option<KodOboru>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uroven: Option<SimulaceMetaUroven>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub podil_zajmu: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skola_obor_uz_uci: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Simulace {
    pub jednotky: BTreeMap<String, SimulacePlocha>,
    pub souhrn: SimulaceSouhrn,
    pub meta: SimulaceMeta,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum SimulaceGeojsonType {
    #[serde(rename = "FeatureCollection")]
    FeatureCollection,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum SimulaceGeojsonFeaturesItemType {
    #[serde(rename = "Feature")]
    Feature,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum SimulaceGeojsonFeaturesItemPropertiesUroven {
    #[serde(rename = "orp")]
    Orp,
    #[serde(rename = "obec")]
    Obec,
    #[serde(rename = "zsj")]
    Zsj,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SimulaceGeojsonFeaturesItemProperties {
    pub kod: KodPlochy,
    pub nazev: String,
    pub uroven: SimulaceGeojsonFeaturesItemPropertiesUroven,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    #[schemars(required)]
    pub cas_min: Option<i64>,
    pub pasmo: Pasmo,
    pub v_dosahu: bool,
    pub deti: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deti_v_dosahu: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub podil_deti_v_dosahu: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nejblizsi_redizo: Option<String>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    #[schemars(required)]
    pub cas_ke_skole: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cas_min_puvodni: Option<i64>,
    pub zlepseni_min: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pasmo_puvodni: Option<Pasmo>,
    pub potencialni_uchazeci: f64,
    pub novy_dosah: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SimulaceGeojsonFeaturesItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<SimulaceGeojsonFeaturesItemType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry: Option<GeoPlocha>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub properties: Option<SimulaceGeojsonFeaturesItemProperties>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SimulaceGeojson {
    pub r#type: SimulaceGeojsonType,
    pub features: Vec<SimulaceGeojsonFeaturesItem>,
    pub souhrn: SimulaceSouhrn,
    pub meta: SimulaceMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Obory {
    pub data: Vec<OborPrehled>,
    pub meta: OboryMeta,
}

/// Catalog-only program listing; accessibility belongs to the analytical endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct OborPrehled {
    pub kod: KodOboru,
    pub nazev: String,
    pub stupen: Stupen,
    pub pocet_skol: i64,
    pub kapacita: i64,
    pub prihlasky: i64,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    #[schemars(required)]
    pub prihlasky_na_misto: Option<f64>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    #[schemars(required)]
    pub index_pretlaku: Option<f64>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    #[schemars(required)]
    pub zamestnavatelu: Option<i64>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    #[schemars(required)]
    pub volna_mista: Option<i64>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    #[schemars(required)]
    pub volna_mista_na_misto: Option<f64>,
    pub signaly: Vec<OborSignal>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum OborSignal {
    #[serde(rename = "pretlak")]
    Pretlak,
    #[serde(rename = "nizky_zajem")]
    NizkyZajem,
    #[serde(rename = "poptavka_trhu")]
    PoptavkaTrhu,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct OboryPrahy {
    pub pretlak: f64,
    pub nizky_zajem: f64,
    pub poptavka_trhu: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct OboryMeta {
    pub prumer_prihlasek_na_misto: f64,
    pub prijimaci_rizeni: MetaBilancePrijimaciRizeni,
    pub prahy: OboryPrahy,
}
