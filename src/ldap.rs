//! Funções relacionadas ao sistema de LDAP usado pela supervisão do LCI para
//! cadastro dos alunos do Instituto de Computação.
use crate::utils::nome::{Nome, NomeErro};
use ldap3::{Ldap, LdapConnAsync, LdapError, Scope, SearchEntry, ldap_escape};
use std::str::FromStr;
use thiserror::Error;

/// Representa um erro ao tentar cadastrar um usuário.
#[derive(Debug, Error)]
pub enum CadastroErro {
    /// Um problema com a conexão com o LDAP. Pode ser um problema de rede ou
    /// um problema com as operações feitas no LDAP. Para saber, acesse a
    /// estrutura [`LdapError`].
    #[error("Houve um problema com o ldap3")]
    ErroLdap(#[from] LdapError),

    /// Houve um erro ao tentar achar o uid de um usuário cujo DRE já está
    /// registrado. Se esse erro foi retornado, significa que o usuário está
    /// cadastrado, mas não se sabe com que nome.
    #[error("Houve um erro ao encontrar o uid do DRE que já está registrado")]
    FalhaUid,

    /// Esse erro acontece quando todos os usernames gerados para um usuário
    /// já estão ocupados. Com a grande quantidade de tentativas, é mais
    /// provável que há um problema com o LDAP ou com o alumnic do que realmente
    /// não ter nome livre. Nesse caso, deve-se verificar se realmente todas
    /// as variações geradas com a função
    /// [`usernames`](crate::utils::nome::Nome::usernames) estão sendo usadas.
    #[error("Não foi possível encontrar um nome de usuário válido")]
    UsuarioDificil,

    /// Houve um problema ao processar o nome retornado pelo Gnosys/SIGA. Isso
    /// significa que, provavelmente, a nossa forma de acessar dados do SIGA
    /// quebrou. Também pode ocorrer caso o usuário tenha um nome "diferente",
    /// ou seja, que não segue as regras para criação de um [`Nome`].
    #[error("Houve um erro ao processar o nome")]
    ErroDeNome(#[from] NomeErro),
}

/// Representa as informações sobre o cadastro de um usuário no LDAP.
#[derive(Debug)]
pub enum Cadastro {
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

async fn rodar_ldap<T, F, Fut>(
    url: &str,
    bind_dn: &str,
    bind_pw: &str,
    f: F,
) -> Result<T, CadastroErro>
where
    F: FnOnce(Ldap) -> Fut,
    Fut: Future<Output = (Result<T, CadastroErro>, Ldap)>,
{
    let (conn, mut ldap) = LdapConnAsync::new(url).await?;
    ldap3::drive!(conn);
    ldap.simple_bind(bind_dn, bind_pw).await?.success()?;

    let (ret, mut ldap) = f(ldap).await;

    ldap.unbind().await?;

    ret
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

    let dre_s = SearchEntry::construct(dre_s.clone());
    let uid = dre_s
        .attrs
        .get("uid")
        .ok_or(CadastroErro::FalhaUid)?
        .first()
        .ok_or(CadastroErro::FalhaUid)?;

    Ok(Some(uid.to_string()))
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

async fn achar_nome_livre(
    nome: &str,
    ldap: &mut Ldap,
) -> Result<String, CadastroErro> {
    for username in Nome::from_str(nome)?.usernames() {
        if !consulta_usuario_existe(&username, ldap).await? {
            return Ok(username);
        }
    }
    Err(CadastroErro::UsuarioDificil)
}
