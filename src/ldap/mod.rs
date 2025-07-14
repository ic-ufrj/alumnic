//! Funções relacionadas ao sistema de LDAP usado pela supervisão do LCI para
//! cadastro dos alunos do Instituto de Computação.

pub mod cadastrar;
pub mod consulta;
pub mod tipos;
pub mod utils;

pub use cadastrar::cadastrar_usuario;
pub use consulta::consultar_cadastro_ldap;
pub use tipos::{Cadastro, CadastroErro};
pub use utils::rodar_ldap;
