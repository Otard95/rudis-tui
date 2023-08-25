use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct RedisServerConf {
    name: String,
    host: String,
    port: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum DBVersions {
    V1_0,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DB_V1_0 {
    pub version: DBVersions,
    pub server_configs: Vec<RedisServerConf>
}

#[derive(Debug)]
pub enum DB {
    DB_V1_0(DB_V1_0),
}
