//! Funções relacionadas ao sistema de LDAP usado pela supervisão do LCI para
//! cadastro dos alunos do Instituto de Computação.

pub mod cadastrar;
pub mod consulta;
pub mod error;
mod utils;

pub use error::{ErroLdap, Result};
