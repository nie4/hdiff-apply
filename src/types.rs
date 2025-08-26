use serde::Deserialize;

#[allow(unused)]
#[derive(Deserialize, Debug, Default)]
pub struct DiffEntry {
    pub source_file_name: String,
    pub source_file_md5: String,
    pub source_file_size: u64,

    pub target_file_name: String,
    pub target_file_md5: String,
    pub target_file_size: u64,

    pub patch_file_name: String,
    pub patch_file_md5: String,
    pub patch_file_size: u64,
}

#[derive(Deserialize, Debug)]
pub struct HDiffMap {
    pub diff_map: Vec<DiffEntry>,
}

#[derive(Deserialize, Debug)]
pub struct CustomDiffMap {
    #[serde(rename = "remoteName")]
    pub remote_name: String,
}
