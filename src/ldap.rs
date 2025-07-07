use ldap3::{LdapConnAsync, LdapError, Scope, SearchEntry, ldap_escape};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConsultaLdapErro {
    #[error("Houve um problema com o ldap3")]
    ErroLdap(#[from] LdapError),

    #[error("Houve um erro ao encontrar o uid do DRE que já está registrado")]
    FalhaUid,
}

#[derive(Debug)]
pub enum ConsultaLdap {
    /// O DRE já está cadastrado, ou seja, o usuário já possui uma conta. Nesse
    /// caso, é retornado o username da conta.
    Registrado(String),
    /// O DRE não está cadastrado, mas o username está sendo usado por algum
    /// outro aluno, professor, etc.
    Conflito,
    /// O DRE não está cadastrado e o username não está sendo usado.
    Disponivel,
}

pub async fn achar_usuario(dre: &str, username: &str) -> Result<ConsultaLdap, ConsultaLdapErro> {
    let bind_dn =
        std::env::var("LDAP_BIND_DN").expect("Por favor forneça uma variável LDAP_BIND_DN");
    let bind_pw =
        std::env::var("LDAP_BIND_PW").expect("Por favor forneça uma variável LDAP_BIND_PW");
    let url = std::env::var("LDAP_URL").expect("Por favor forneça uma variável LDAP_URL");

    let search_dre = format!("(dre={})", ldap_escape(dre));
    let search_username = format!("(uid={})", ldap_escape(username));

    let (conn, mut ldap) = LdapConnAsync::new(&url).await?;

    ldap3::drive!(conn);

    ldap.simple_bind(&bind_dn, &bind_pw).await?.success()?;

    let (dre_s, _) = ldap
        .search(
            "dc=dcc,dc=ufrj,dc=br",
            Scope::Subtree,
            &search_dre,
            vec!["uid"],
        )
        .await?
        .success()?;

    if let Some(dre_s) = dre_s.first() {
        ldap.unbind().await?;

        let dre_s = SearchEntry::construct(dre_s.clone());
        let uid = dre_s
            .attrs
            .get("uid")
            .ok_or(ConsultaLdapErro::FalhaUid)?
            .first()
            .ok_or(ConsultaLdapErro::FalhaUid)?;

        return Ok(ConsultaLdap::Registrado(uid.to_string()));
    }

    let (username_s, _) = ldap
        .search(
            "dc=dcc,dc=ufrj,dc=br",
            Scope::Subtree,
            &search_username,
            Vec::<&str>::new(),
        )
        .await?
        .success()?;

    ldap.unbind().await?;

    if username_s.is_empty() {
        Ok(ConsultaLdap::Disponivel)
    } else {
        Ok(ConsultaLdap::Conflito)
    }
}
