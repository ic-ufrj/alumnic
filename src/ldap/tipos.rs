use crate::utils::nome::NomeErro;
use ldap3::LdapError;
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
    /// ou seja, que não segue as regras para criação de um
    /// [`Nome`](crate::utils::nome::Nome).
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
