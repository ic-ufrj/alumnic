use config::{Config, ConfigError, File};
use directories::ProjectDirs;
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Deserialize)]
pub struct Configuracao {
    pub ldap_url: String,
    pub ldap_bind_dn: String,
    pub ldap_bind_pw: String,
}

#[derive(Debug, Error)]
pub enum ConfiguracaoErro {
    #[error("Não foi possível encontrar o diretório de configuração")]
    ProjectDirs,
    #[error(transparent)]
    ErroNaConfig(#[from] ConfigError),
}

impl Configuracao {
    pub fn importar() -> Result<Self, ConfiguracaoErro> {
        let arquivo_de_config = ProjectDirs::from("br", "ufrj.ic", "alumnic")
            .ok_or(ConfiguracaoErro::ProjectDirs)?
            .config_dir()
            .to_path_buf()
            .join("config.yaml");

        Ok(Config::builder()
            .add_source(File::from(arquivo_de_config))
            .build()?
            .try_deserialize()?)
    }
}
