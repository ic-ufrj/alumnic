use crate::cadastro_aluno::DadosParaCadastro;
use crate::configuracao::ConfiguracaoUsuario;
use crate::ldap::ErroLdap;
use crate::ldap::utils::rodar_ldap;
use crate::utils::hashes::{hash_nt, hash_ssha};
use chrono::Utc;
use deunicode::deunicode;
use ldap3::{Ldap, Mod, Scope, SearchEntry, dn_escape};
use secrecy::ExposeSecret;

// TODO: documentar que é possível que uma race condition aconteça caso dois
// usuários disputem um mesmo username ao mesmo tempo, mas nesse caso, o LDAP
// retornará um erro, o que não é algo crítico, só seria necessário que o
// usuário tente novamente. Como é um caso extremamente excepcional, não acho
// que isso seja um problema.
pub async fn cadastrar_usuario(
    username: String,
    dados: &DadosParaCadastro,
    cfg: &ConfiguracaoUsuario,
    ldap_url: &str,
    bind_dn: &str,
    bind_pw: &str,
) -> Result<(), ErroLdap> {
    async fn cadastrar(
        username: String,
        dados: &DadosParaCadastro,
        cfg: &ConfiguracaoUsuario,
        ldap: &mut Ldap,
    ) -> Result<(), ErroLdap> {
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
        (cadastrar(username, dados, cfg, &mut ldap).await, ldap)
    })
    .await
}

async fn samba_ids(ldap: &mut Ldap) -> Result<(String, String), ErroLdap> {
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
        return Err(ErroLdap::ErroSamba);
    };

    // TODO: precisa desse clone?
    let ids_s = SearchEntry::construct(ids_s.clone());

    let samba_uid = ids_s
        .attrs
        .get("uidNumber")
        .and_then(|x| x.first())
        .ok_or(ErroLdap::ErroSamba)?;

    let samba_rid = ids_s
        .attrs
        .get("sambaNextRid")
        .and_then(|x| x.first())
        .ok_or(ErroLdap::ErroSamba)?;

    let prox_samba_uid =
        (samba_uid.parse::<i64>().map_err(|_| ErroLdap::ErroSamba)? + 1)
            .to_string();
    let prox_samba_rid =
        (samba_rid.parse::<i64>().map_err(|_| ErroLdap::ErroSamba)? + 1)
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
    Err(ErroLdap::ErroSamba)
}
