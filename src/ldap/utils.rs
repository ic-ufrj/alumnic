use crate::ldap::CadastroErro;
use ldap3::{Ldap, LdapConnAsync};

pub async fn rodar_ldap<T, F, Fut>(
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
