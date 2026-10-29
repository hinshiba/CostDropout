//! 帳簿のファイル形式に合わせたシリアライズとデシリアライズ

/// `NaiveDate` を TOML の日付型で読み書きする.
///
/// `#[serde(with = "crate::serialization::toml_date")]` で使う.
pub(crate) mod toml_date {
    use chrono::NaiveDate;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use toml::value::Datetime;

    pub fn serialize<S: Serializer>(date: &NaiveDate, serializer: S) -> Result<S::Ok, S::Error> {
        let datetime = date
            .to_string()
            .parse::<Datetime>()
            .map_err(serde::ser::Error::custom)?;
        datetime.serialize(serializer)
    }

    /// # Errors
    ///
    /// TOML の日付型でない場合と, 時刻を含む場合はエラー.
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<NaiveDate, D::Error> {
        Datetime::deserialize(deserializer)?
            .to_string()
            .parse::<NaiveDate>()
            .map_err(serde::de::Error::custom)
    }
}
