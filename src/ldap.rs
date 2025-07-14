//! Funções relacionadas ao sistema de LDAP usado pela supervisão do LCI para
//! cadastro dos alunos do Instituto de Computação.
use crate::cadastro_aluno::DadosParaCadastro;
use crate::configuracao::ConfiguracaoUsuario;
use crate::utils::nome::{Nome, NomeErro};
use base64::prelude::*;
use chrono::Utc;
use deunicode::deunicode;
use encoding::all::UTF_16LE;
use encoding::{EncoderTrap, Encoding};
use ldap3::dn_escape;
use ldap3::{
    Ldap, LdapConnAsync, LdapError, Mod, Scope, SearchEntry, ldap_escape,
};
use md4::Md4;
use rand::Rng;
use secrecy::{ExposeSecret, SecretString};
use sha1::{Digest, Sha1};
use std::str::FromStr;
use thiserror::Error;
use zeroize::Zeroize;

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

    #[error("Houve um erro ao tentar criar os IDs do Samba")]
    ErroSamba,
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

// TODO: documentar que é possível que uma race condition aconteça caso dois
// usuários disputem um mesmo username ao mesmo tempo, mas nesse caso, o LDAP
// retornará um erro, o que não é algo crítico, só seria necessário que o
// usuário tente novamente. Como é um caso extremamente excepcional, não acho
// que isso seja um problema.
//
// TODO: anotar que dá panic quando o cadastro n é disponivel
pub async fn cadastrar_usuario(
    cadastro: Cadastro,
    dados: &DadosParaCadastro,
    cfg: &ConfiguracaoUsuario,
    ldap_url: &str,
    bind_dn: &str,
    bind_pw: &str,
) -> Result<(), CadastroErro> {
    async fn cadastrar(
        cadastro: Cadastro,
        dados: &DadosParaCadastro,
        cfg: &ConfiguracaoUsuario,
        ldap: &mut Ldap,
    ) -> Result<(), CadastroErro> {
        let username = match cadastro {
            Cadastro::CadastroDisponivel(uid) => uid,
            Cadastro::CadastroRedundante(..) => {
                panic!("Cadastro precisa estar disponível!")
            },
        };

        let (samba_uid, samba_rid) = samba_ids(ldap).await?;

        let dn = format!(
            "uid={},ou=alunos,ou=academicos,ou=usuarios,dc=dcc,dc=ufrj,dc=br",
            dn_escape(&username),
        );

        let hash_nt = hash_nt(&dados.senha);
        let hash_ssha = hash_ssha(&dados.senha);

        // Hoje no tempo UNIX
        let samba_today = Utc::now().timestamp();
        // + 10 anos
        let samba_kickoff = samba_today + (3600 * 24 * 60 * 60);
        // De segundos para dias
        let shadow_today = samba_today / (24 * 60 * 60);
        // + 10 anos
        let shadow_renovacao = shadow_today + 3600;
        // Converte tudo para String
        let (samba_today, samba_kickoff, shadow_today, shadow_renovacao) = (
            samba_today.to_string(),
            samba_kickoff.to_string(),
            shadow_today.to_string(),
            shadow_renovacao.to_string(),
        );

        ldap.add(
            &dn,
            vec![
                (
                    "objectClass",
                    [
                        "dcc",
                        "dccAluno",
                        "sambaSamAccount",
                        "shadowAccount",
                        "posixAccount",
                        "inetOrgPerson",
                    ]
                    .into(),
                ),
                ("dccDRE", [dados.dre.as_str()].into()),
                ("gidNumber", [cfg.gid_number.as_str()].into()),
                (
                    "homeDirectory",
                    [format!("/usuarios/alunos/{username}").as_str()].into(),
                ),
                (
                    "sambaSID",
                    [format!("{}{samba_rid}", cfg.samba_sid_prefix).as_str()]
                        .into(),
                ),
                ("uid", [username.as_str()].into()),
                ("mail", [format!("{username}@dcc.ufrj.br").as_str()].into()),
                ("uidNumber", [samba_uid.as_str()].into()),
                ("gecos", [deunicode(&dados.nome).as_str()].into()),
                ("cn", [dados.nome.split_whitespace().next().unwrap()].into()),
                (
                    "sn",
                    [dados
                        .nome
                        .split_whitespace()
                        .skip(1)
                        .collect::<Vec<_>>()
                        .join(" ")
                        .as_str()]
                    .into(),
                ),
                ("loginShell", ["/bin/bash"].into()),
                ("emailExterno", [dados.email.as_str()].into()),
                /* SAMBA - relacionado ao samba, desativado no momento */
                ("sambaAcctFlags", [cfg.samba_acct_flags.as_str()].into()),
                ("sambaKickoffTime", [samba_kickoff.as_str()].into()),
                ("sambaLMPassword", [cfg.samba_lm_password.as_str()].into()),
                ("sambaNTPassword", [hash_nt.expose_secret()].into()),
                (
                    "sambaPasswordHistory",
                    [cfg.samba_password_history.as_str()].into(),
                ),
                (
                    "sambaPrimaryGroupSID",
                    [cfg.samba_primary_group_sid.as_str()].into(),
                ),
                ("sambaPwdLastSet", [samba_today.as_str()].into()),
                ("sambaPwdMustChange", [samba_kickoff.as_str()].into()),
                /* SHADOW - relacionado ao login nos laboratórios */
                // O acesso aos laboratórios não expira
                ("shadowExpire", ["-1"].into()),
                // Parece ser sempre -1
                ("shadowFlag", ["-1"].into()),
                // Desabilita bloqueio da conta após a senha expirar
                ("shadowInactive", ["-1"].into()),
                // Data da última troca de senha
                ("shadowLastChange", [shadow_today.as_str()].into()),
                // Vencimento das senhas após 10 anos
                ("shadowMax", ["3600"].into()),
                // A senha pode ser trocada a qualquer momento.
                ("shadowMin", ["0"].into()),
                // Quanto tempo antes da expiração da senha alertar o usuário
                ("shadowWarning", ["14"].into()),
                ("telephoneNumber", [dados.telefone.as_str()].into()),
                ("userPassword", [hash_ssha.expose_secret()].into()),
                ("cota", [cfg.cota.as_str()].into()),
                ("monitor", ["0"].into()),
                ("dataCriacao", [shadow_today.as_str()].into()),
                ("dataRenovacao", [shadow_renovacao.as_str()].into()),
            ],
        )
        .await?
        .success()?;

        Ok(())
    }
    rodar_ldap(ldap_url, bind_dn, bind_pw, |mut ldap| async move {
        (cadastrar(cadastro, dados, cfg, &mut ldap).await, ldap)
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

fn hash_nt(passwd: &SecretString) -> SecretString {
    let mut passwd_utf16le = UTF_16LE
        .encode(passwd.expose_secret(), EncoderTrap::Strict)
        .unwrap();
    let mut hasher = Md4::new();
    hasher.update(&passwd_utf16le);
    let r: SecretString = hex::encode_upper(hasher.finalize()).into();

    passwd_utf16le.zeroize();

    r
}

fn hash_ssha(passwd: &SecretString) -> SecretString {
    let mut salt = [0u8; 4];
    rand::rng().fill(&mut salt);

    let mut hasher = Sha1::new();
    hasher.update(passwd.expose_secret().as_bytes());
    hasher.update(&salt);
    let mut hash = hasher.finalize();

    let mut salted = BASE64_STANDARD.encode([hash.as_slice(), &salt].concat());

    let r: SecretString = format!("{}{}", "{{SSHA}}", salted).into();

    salt.zeroize();
    hash.zeroize();
    salted.zeroize();

    r
}

async fn samba_ids(ldap: &mut Ldap) -> Result<(String, String), CadastroErro> {
    let (ids_s, _) = ldap
        .search(
            "dc=dcc,dc=ufrj,dc=br",
            Scope::OneLevel,
            "(objectClass=sambaDomain)",
            vec!["uidNumber", "sambaNextRid"],
        )
        .await?
        .success()?;

    let Some(ids_s) = ids_s.first() else {
        return Err(CadastroErro::ErroSamba);
    };

    // TODO: precisa desse clone?
    let ids_s = SearchEntry::construct(ids_s.clone());

    let samba_uid = ids_s
        .attrs
        .get("uidNumber")
        .and_then(|x| x.first())
        .ok_or(CadastroErro::ErroSamba)?;

    let samba_rid = ids_s
        .attrs
        .get("sambaNextRid")
        .and_then(|x| x.first())
        .ok_or(CadastroErro::ErroSamba)?;

    let prox_samba_uid = (samba_uid
        .parse::<i64>()
        .map_err(|_| CadastroErro::ErroSamba)?
        + 1)
    .to_string();
    let prox_samba_rid = (samba_rid
        .parse::<i64>()
        .map_err(|_| CadastroErro::ErroSamba)?
        + 1)
    .to_string();

    for _ in 1..=5 {
        let modificacao = ldap
            .modify(
                &ids_s.dn,
                vec![
                    Mod::Delete("uidNumber", [samba_uid.as_str()].into()),
                    Mod::Add("uidNumber", [prox_samba_uid.as_str()].into()),
                    Mod::Delete("sambaNextRid", [samba_rid.as_str()].into()),
                    Mod::Add("sambaNextRid", [prox_samba_rid.as_str()].into()),
                ],
            )
            .await
            .and_then(|x| x.success());

        if modificacao.is_ok() {
            return Ok((prox_samba_uid, prox_samba_rid));
        }
    }
    Err(CadastroErro::ErroSamba)
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
