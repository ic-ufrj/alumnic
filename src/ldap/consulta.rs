use crate::ldap::{Cadastro, CadastroErro, rodar_ldap};
use crate::utils::nome::Nome;
use ldap3::{Ldap, Scope, SearchEntry, ldap_escape};

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
/// Mais informações em [CadastroErro]
pub async fn consultar_cadastro_ldap(
    dre: &str,
    nome: &str,
    ldap_url: &str,
    bind_dn: &str,
    bind_pw: &str,
) -> Result<Cadastro, CadastroErro> {
    rodar_ldap(ldap_url, bind_dn, bind_pw, |mut ldap| async move {
        match consulta_dre(dre, &mut ldap).await {
            Err(err) => (Err(err), ldap),
            Ok(Some(uid)) => (Ok(Cadastro::CadastroRedundante(uid)), ldap),
            Ok(None) => match achar_nome_livre(nome, &mut ldap).await {
                Err(err) => (Err(err), ldap),
                Ok(uid) => (Ok(Cadastro::CadastroDisponivel(uid)), ldap),
            },
        }
    })
    .await
}

async fn consulta_dre(
    dre: &str,
    ldap: &mut Ldap,
) -> Result<Option<String>, CadastroErro> {
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
        .ok_or(CadastroErro::FalhaUid)?
        .first()
        .ok_or(CadastroErro::FalhaUid)?;

    Ok(Some(uid.to_string()))
}

async fn achar_nome_livre(
    nome: &str,
    ldap: &mut Ldap,
) -> Result<String, CadastroErro> {
    for username in nome.parse::<Nome>()?.usernames() {
        if !consulta_usuario_existe(&username, ldap).await? {
            return Ok(username);
        }
    }
    Err(CadastroErro::UsuarioDificil)
}

async fn consulta_usuario_existe(
    username: &str,
    ldap: &mut Ldap,
) -> Result<bool, CadastroErro> {
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
