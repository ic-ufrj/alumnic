use crate::ldap::ErroLdap;
use crate::ldap::utils::rodar_ldap;
use crate::utils::nome::Nome;
use ldap3::{Ldap, Scope, SearchEntry, ldap_escape};

/// Representa as informações sobre o cadastro de um usuário no LDAP.
#[derive(Debug)]
pub enum Consulta {
    /// O cadastro pode ser feito com sucesso e a string representa o
    /// uid/sername do usuário que deve ser criado.
    CadastroDisponivel(String),
    /// O cadastro já existia antes. A string representa o username/uid do
    /// usuário **que já estava cadastrado**.
    CadastroRedundante(String),
}

/// Consulta se um usuário já está cadastrado no LDAP a partir da DRE e, se ele
/// não estiver, acha um uid/username disponível para ele. Se ele estiver, diz
/// qual uid/username o usuário tem cadastrado.
///
/// # Errors
///
/// Retorna erro caso ocorra um problema ao se comunicar com o LDAP, caso não
/// consiga achar o uid que o usuário tem, caso não consiga gerar um username
/// válido ou caso o nome do usuário não seja válido.
///
/// Mais informações em [ErroLdap]
pub async fn consultar_cadastro_ldap(
    dre: &str,
    nome: &str,
    ldap_url: &str,
    bind_dn: &str,
    bind_pw: &str,
) -> Result<Consulta, ErroLdap> {
    rodar_ldap(ldap_url, bind_dn, bind_pw, |mut ldap| async move {
        match consulta_dre(dre, &mut ldap).await {
            Err(err) => (Err(err), ldap),
            Ok(Some(uid)) => (Ok(Consulta::CadastroRedundante(uid)), ldap),
            Ok(None) => match achar_nome_livre(nome, &mut ldap).await {
                Err(err) => (Err(err), ldap),
                Ok(uid) => (Ok(Consulta::CadastroDisponivel(uid)), ldap),
            },
        }
    })
    .await
}

async fn consulta_dre(
    dre: &str,
    ldap: &mut Ldap,
) -> Result<Option<String>, ErroLdap> {
    let search_dre = format!("(dre={})", ldap_escape(dre));

    let (dre_s, _) = ldap
        .search(
            "dc=dcc,dc=ufrj,dc=br",
            Scope::Subtree,
            &search_dre,
            vec!["uid"],
        )
        .await?
        .success()?;

    let Some(dre_s) = dre_s.first() else {
        return Ok(None);
    };

    // TODO: precisa desse clone?
    let dre_s = SearchEntry::construct(dre_s.clone());
    let uid = dre_s
        .attrs
        .get("uid")
        .ok_or(ErroLdap::FalhaUid)?
        .first()
        .ok_or(ErroLdap::FalhaUid)?;

    Ok(Some(uid.to_string()))
}

async fn achar_nome_livre(
    nome: &str,
    ldap: &mut Ldap,
) -> Result<String, ErroLdap> {
    for username in nome.parse::<Nome>()?.usernames() {
        if !consulta_usuario_existe(&username, ldap).await? {
            return Ok(username);
        }
    }
    Err(ErroLdap::UsuarioDificil)
}

async fn consulta_usuario_existe(
    username: &str,
    ldap: &mut Ldap,
) -> Result<bool, ErroLdap> {
    let search_username = format!("(uid={})", ldap_escape(username));

    let (username_s, _) = ldap
        .search(
            "dc=dcc,dc=ufrj,dc=br",
            Scope::Subtree,
            &search_username,
            Vec::<&str>::new(),
        )
        .await?
        .success()?;

    Ok(!username_s.is_empty())
}
