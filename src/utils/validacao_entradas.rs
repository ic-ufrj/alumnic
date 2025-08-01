//! Módulo para validar os dados de cadastro de um aluno, garantindo não só que
//! não há entradas maliciosas, mas também que os dados foram corretamente
//! preenchidos e que não houve erros por parte de um usuário bem-intencionado.
//! Também ajuda a converter informações que possuem várias representações para
//! a representação "padrão" usada pelo SIGA e por nosso sistema de LDAP.
use crate::utils::nome::Nome;
use email_address::EmailAddress;
use regex::Regex;
use secrecy::{ExposeSecret, SecretString};

/// Processa um DRE, retornando uma versão "limpa" dele caso a entrada seja
/// válida e None caso a entrada não represente um DRE válido.
///
/// Os DREs válidos são sequências de nove dígitos com ou sem espaço antes ou depois delas.
///
/// # Examples
///
/// ```
/// # use alumnic::utils::validacao_entradas::processar_dre;
/// assert_eq!(processar_dre("123456789"), Some("123456789".to_string()));
/// assert_eq!(processar_dre("345678912 "), Some("345678912".to_string()));
/// assert_eq!(processar_dre(" 34s333333"), None);
/// assert_eq!(processar_dre("12345678 "), None);
/// ```
pub fn processar_dre(dre: &str) -> Option<String> {
    let re = Regex::new(r"^\s*(\d{9})\s*$").unwrap();

    re.captures(dre).map(|caps| format!("{}", &caps[1]))
}

/// Processa uma data de emissão, convertendo ela para o formato "dd/mm/aaaa"
/// caso consiga processar ela e retornando None se não conseguir.
///
/// Uma data válida pode ser:
///
/// - separada por barras, contendo um ou dois dígitos para o dia, um ou dois
///   dígitos para o mês e de 1 a 4 dígitos para o ano. Caso tenha menos de 4
///   dígitos, o número 2000 é adicionado, ou seja, "25" vira "2025" e "100"
///   vira "2100"; ou
/// - separada por espaços ou por nada, nesse caso o dia e mês precisam ter
///   sempre dois dígitos, com zero à esquerda caso seja menor que 10 e o ano
///   precisa ter, obrigatoriamente, 4 dígitos.
///
/// Em ambos os casos, espaços entre os componentes são aceitos, exceto se for
/// entre os dígitos de um único número.
///
/// # Examples
///
/// ```
/// # use alumnic::utils::validacao_entradas::processar_data;
/// assert_eq!(processar_data("01/01/2025"), Some("01/01/2025".to_string()));
/// assert_eq!(processar_data("1 / 1 / 25"), Some("01/01/2025".to_string()));
/// assert_eq!(processar_data("01012025"), Some("01/01/2025".to_string()));
/// assert_eq!(processar_data("25 12 2002"), Some("25/12/2002".to_string()));
/// assert_eq!(processar_data("25 12 02"), None);
/// assert_eq!(processar_data("1 1 2002"), None);
/// assert_eq!(processar_data("25/12/02"), Some("25/12/2002".to_string()));
/// ```
pub fn processar_data(data: &str) -> Option<String> {
    // Strings do tipo "1/1/2025", "1/1/25", "01/01/2025", etc.
    let re1 = Regex::new(r"^\s*(\d{1,2})\s*/\s*(\d{1,2})\s*/\s*(\d{1,4})\s*$")
        .unwrap();
    // Strings do tipo "01012025", "0101 25", etc.
    let re2 = Regex::new(r"^\s*(\d{2})\s*(\d{2})\s*(\d{4})\s*$").unwrap();

    re1.captures(data)
        // Testa a segunda expressão se a primeira falhar
        .or_else(move || re2.captures(data))
        .map(|caps| {
            format!(
                "{:02}/{:02}/{}",
                caps[1].parse::<u8>().unwrap(),
                caps[2].parse::<u8>().unwrap(),
                // Adiciona o 2000 se for um número de três dígitos
                match caps[3].parse::<u16>().unwrap() {
                    x if x < 1000 => 2000 + x,
                    x => x,
                },
            )
        })
}

/// Processa uma hora de emissão, retirando espaços adicionais entre os números.
///
/// # Examples
///
/// ```
/// # use alumnic::utils::validacao_entradas::processar_hora;
/// assert_eq!(processar_hora("16 :2"), Some("16:02".to_string()));
/// assert_eq!(processar_hora("9:5"), Some("09:05".to_string()));
/// assert_eq!(processar_hora("23:59"), Some("23:59".to_string()));
/// assert_eq!(processar_hora("00:00"), Some("00:00".to_string()));
/// assert_eq!(processar_hora("7 : 8"), Some("07:08".to_string()));
/// assert_eq!(processar_hora(" 2 : 3 "), Some("02:03".to_string()));
/// assert_eq!(processar_hora("1:23"), Some("01:23".to_string()));
/// assert_eq!(processar_hora("12:7"), Some("12:07".to_string()));
/// // Não há uma verificação muito detalhada das horas, basta ser dois números
/// // que é o suficiente.
/// assert_eq!(processar_hora("24:00"), Some("24:00".to_string()));
/// assert_eq!(processar_hora("12:60"), Some("12:60".to_string()));
/// assert_eq!(processar_hora("::"), None);
/// assert_eq!(processar_hora("abc"), None);
/// assert_eq!(processar_hora("12:34:56"), None);
/// assert_eq!(processar_hora(""), None);
/// assert_eq!(processar_hora("  "), None);
/// ```
pub fn processar_hora(hora: &str) -> Option<String> {
    let re = Regex::new(r"^\s*(\d{1,2})\s*\:\s*(\d{1,2})\s*$").unwrap();

    re.captures(hora).map(|caps| {
        format!(
            "{:02}:{:02}",
            &caps[1].parse::<u8>().unwrap(),
            &caps[2].parse::<u8>().unwrap()
        )
    })
}

/// Processa um dos códigos gerados pelo SIGA para autenticação do documento de
/// regularmente matriculado. Aceita espaços entre os segmentos e no começo e
/// final do código e converte para um código mais limpo. Não aceita caracteres
/// que não fazem parte de um número hexadecimal.
///
/// # Examples
///
/// ```
/// # use alumnic::utils::validacao_entradas::processar_codigo;
/// assert_eq!(
///     processar_codigo("A3B1.7E5D.F002.19AC.4F6B.9D3E.82C1.BAAF"),
///     Some("A3B1.7E5D.F002.19AC.4F6B.9D3E.82C1.BAAF".to_string()),
/// );
///
/// // Minúsculas = inválido.
/// assert_eq!(
///     processar_codigo("a3b1.7e5d.f002.19ac.4f6b.9d3e.82c1.baaf"),
///     None,
/// );
///
/// // Número de segmentos diferente de oito = inválido.
/// assert_eq!(
///     processar_codigo("A3B1.7E5D.F002.19AC.4F6B.9D3E.82C1"),
///     None,
/// );
/// assert_eq!(
///     processar_codigo("A3B1.7E5D.F002.19AC.4F6B.9D3E.82C1.BAAF.1234"),
///     None,
/// );
///
/// // Caractere não hexadecimal = inválido.
/// assert_eq!(
///     processar_codigo("A3B1.7E5D.F002.19AC.4F6B.9D3E.82C1.ZZZZ"),
///     None,
/// );
///
/// // Dois pontos seguidos = inválido.
/// assert_eq!(
///     processar_codigo("A3B1..7E5D.F002.19AC.4F6B.9D3E.82C1.BAAF"),
///     None,
/// );
///
/// // Obviamente inválido.
/// assert_eq!(processar_codigo(""), None);
///
/// // Espaços não devem ser um problema.
/// assert_eq!(
///     processar_codigo(" A3B1 . 7E5D  .F002.19AC.4F6B.9D3E.82C1.BAAF "),
///     Some("A3B1.7E5D.F002.19AC.4F6B.9D3E.82C1.BAAF".to_string()),
/// );
///
/// // Segmento com número de caracteres diferente de quatro.
/// assert_eq!(
///     processar_codigo("A3B1.7E5D.F02.19AC.4F6B.9D3E.82C1.BAAF"),
///     None
/// );
/// ```
pub fn processar_codigo(codigo: &str) -> Option<String> {
    let re = Regex::new(concat!(
        r"^\s*([0-9A-F]{4})",
        r"\s*\.\s*([0-9A-F]{4})",
        r"\s*\.\s*([0-9A-F]{4})",
        r"\s*\.\s*([0-9A-F]{4})",
        r"\s*\.\s*([0-9A-F]{4})",
        r"\s*\.\s*([0-9A-F]{4})",
        r"\s*\.\s*([0-9A-F]{4})",
        r"\s*\.\s*([0-9A-F]{4})\s*$",
    ))
    .unwrap();

    re.captures(codigo).map(|caps| {
        format!(
            "{}.{}.{}.{}.{}.{}.{}.{}",
            &caps[1],
            &caps[2],
            &caps[3],
            &caps[4],
            &caps[5],
            &caps[6],
            &caps[7],
            &caps[8],
        )
    })
}

/// Processa um nome, retornando sua versão com cada palavra com a primeira
/// letra maiúscula, exceto as palavras "de", "da", "das", "do" e "dos", que
/// ficam todas minúsculas.
///
/// Nomes válidos possuem no mínimo duas palavras, sem contar com "de", "da",
/// etc. e somente possuem letras e espaços.
///
/// # Examples
///
/// ```
/// # use alumnic::utils::validacao_entradas::processar_nome;
/// // Nomes são capitalizados automaticamente
/// assert_eq!(
///     processar_nome("josé da     silva"),
///     Some("José da Silva".to_string())
/// );
///
/// // Nomes não podem ter caracteres inválidos
/// assert_eq!(processar_nome("maria123 de souza"), None);
///
/// // Nomes precisam ter ao menos uma palavra ("de" não conta como palavra)
/// assert_eq!(processar_nome("de souza"), None);
/// ```
pub fn processar_nome(nome: &str) -> Option<String> {
    // Verifica se o nome é válido
    nome.parse::<Nome>().ok()?;

    Some(
        // Primeiro, converte o nome para todo minúsculo
        nome.to_lowercase()
            // Separa em palavras
            .split_whitespace()
            .map(|x| {
                // Mantém a palavra toda minúscula se for uma dessas
                if ["de", "da", "do", "das", "dos"].contains(&x) {
                    x.to_string()
                } else {
                    // Capitaliza a palavra, primeiro pegando o primeiro
                    // caractere
                    x.chars()
                        .next()
                        .unwrap()
                        // Transformando ele em maiúsculo, o que retorna uma
                        // lista de caracteres (o motivo disso é explicado na
                        // documentação da função char::to_uppercase, da std do
                        // Rust)
                        .to_uppercase()
                        // Junta esse iterador do primeiro caractere maiúsculo
                        // com os próximos caracteres (que continuam minúsculos)
                        .chain(x.chars().skip(1))
                        // Transforma esse iterador de caracteres em uma String
                        .collect::<String>()
                }
            })
            // Coleta esse iterador de Strings em um vetor de Strings
            .collect::<Vec<_>>()
            // Junta tudo com um espaço entre as palavras
            .join(" "),
    )
}

/// Processar/normalizar um endereço de email.
///
/// - Converte para minúsculas;
/// - Remove espaços em volta; e
/// - Retorna `None` se o formato for inválido.
///
/// # Examples
///
/// ```
/// # use alumnic::utils::validacao_entradas::processar_email;
/// // Email válido
/// assert_eq!(
///     processar_email("  JoSe@Exemplo.Com  "),
///     Some("JoSe@exemplo.com".to_string())
/// );
///
/// // Email válido com nome (que é ignorado)
/// assert_eq!(
///     processar_email("Maria <maria@exemplo.com>"),
///     Some("maria@exemplo.com".to_string())
/// );
///
/// // Caractere ! é válido
/// assert_eq!(
///     processar_email("joão!@email.com"),
///     Some("joão!@email.com".to_string()),
/// );
///
/// // Faltando o domínio
/// assert_eq!(processar_email("jose@"), None);
///
/// // Sem arroba
/// assert_eq!(processar_email("jose.email.com"), None);
///
/// // Duplo arroba
/// assert_eq!(processar_email("jose@joao@email.com"), None);
///
/// // Duplo arroba válido
/// assert_eq!(
///     processar_email(r#""jose@joao"@email.com"#),
///     Some(r#""jose@joao"@email.com"#.to_string()),
/// );
/// ```
pub fn processar_email(email: &str) -> Option<String> {
    // Tira espaços extras entre o email
    let email: EmailAddress = email.trim().parse().ok()?;
    // Faz o domínio do email ficar minúsculo e monta um email novo
    let email =
        format!("{}@{}", email.local_part(), email.domain().to_lowercase());
    // Processa esse email formado, para ter 100% de certeza que é válido
    let email: EmailAddress = email.parse().ok()?;
    // Transforma ele em String novamente
    Some(email.to_string())
}

/// Processa/normaliza números de telefone para um formato semelhante a
/// `+5521987654321` ou `+552112345678` para números fixos
///
/// - Remove espaços, hífens, parênteses e `0` inicial no DDD;
/// - Aceita números fixos e celulares; e
/// - Retorna `None` se não for um número brasileiro válido.
///
/// # Examples
///
/// ```
/// # use alumnic::utils::validacao_entradas::processar_telefone;
/// // Celular com DDD e +55
/// assert_eq!(
///     processar_telefone("+55 (21) 98765-4321"),
///     Some("+5521987654321".to_string())
/// );
///
/// // Celular sem código de país
/// assert_eq!(
///     processar_telefone("(21) 98765-4321"),
///     Some("+5521987654321".to_string())
/// );
///
/// // Celular com DDD "021" (com zero)
/// assert_eq!(
///     processar_telefone("021 98765-4321"),
///     Some("+5521987654321".to_string())
/// );
///
/// // Celular sem parênteses
/// assert_eq!(
///     processar_telefone("21 987654321"),
///     Some("+5521987654321".to_string())
/// );
///
/// // Celular com espaços extras
/// assert_eq!(
///     processar_telefone(" 21  98765 - 4321 "),
///     Some("+5521987654321".to_string())
/// );
///
/// // Número fixo
/// assert_eq!(
///     processar_telefone("21 2345-6789"),
///     Some("+552123456789".to_string())
/// );
///
/// // DDD com zero de novo
/// assert_eq!(
///     processar_telefone("(085) 98765-4321"),
///     Some("+5585987654321".to_string())
/// );
///
/// // Sem DDD (considerado inválido)
/// assert_eq!(processar_telefone("98765-4321"), None);
///
/// // Com caracteres inválidos
/// assert_eq!(processar_telefone("telefone: (21) 98765-43!1"), None);
///
/// // Número muito curto
/// assert_eq!(processar_telefone("12345"), None);
///
/// // Número muito longo
/// assert_eq!(processar_telefone("+55 (21) 98765-432100000"), None);
///
/// // Somente números
/// assert_eq!(
///     processar_telefone("21987654321"),
///     Some("+5521987654321".to_string()),
/// );
/// ```
pub fn processar_telefone(telefone: &str) -> Option<String> {
    let re = Regex::new(
        r"^\s*(?:\+55)?\s*\(?0?(\d\d)\)?\s*(9?\d{4})\s*\-?\s*(\d{4})\s*$",
    )
    .unwrap();

    re.captures(telefone)
        .map(|caps| format!("+55{}{}{}", &caps[1], &caps[2], &caps[3]))
}

/// Valida uma senha representada com os tipos da biblioteca [secrecy].
///
/// As condições para uma senha ser válida são:
///
/// - ter ao menos 8 caracteres;
/// - ter no máximo 25 caracteres;
/// - ter ao menos uma letra minúscula;
/// - ter ao menos uma letra maiúscula; e
/// - ter ao menos um dígito.
pub fn validar_senha(senha: &SecretString) -> bool {
    let s = senha.expose_secret();

    s.chars().count() >= 8
        && s.chars().count() <= 25
        && s.chars().any(|c| c.is_ascii_lowercase())
        && s.chars().any(|c| c.is_ascii_uppercase())
        && s.chars().any(|c| c.is_ascii_digit())
}
