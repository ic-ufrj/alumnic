//! Módulo para validar os dados de cadastro de um aluno, garantindo não só que
//! não há entradas maliciosas, mas também que os dados foram corretamente
//! preenchidos e que não houve erros por parte de um usuário bem-intencionado.
//! Também ajuda a converter informações que possuem várias representações para
//! a representação "padrão" usada pelo SIGA e por nosso sistema de LDAP.
use crate::utils::nome::Nome;
use email_address::EmailAddress;
use regex::Regex;

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
        .or_else(move || re2.captures(data))
        .map(|caps| {
            format!(
                "{:02}/{:02}/{}",
                caps[1].parse::<u8>().unwrap(),
                caps[2].parse::<u8>().unwrap(),
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
        nome.to_lowercase()
            .split_whitespace()
            .filter(|x| !x.is_empty())
            .map(|x| {
                if ["de", "da", "do", "das", "dos"].contains(&x) {
                    x.to_string()
                } else {
                    x.chars()
                        .next()
                        .unwrap()
                        .to_uppercase()
                        .chain(x.chars().skip(1))
                        .collect::<String>()
                }
            })
            .collect::<Vec<_>>()
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
    let email: EmailAddress = email.trim().parse().ok()?;
    let email =
        format!("{}@{}", email.local_part(), email.domain().to_lowercase());
    let email: EmailAddress = email.parse().ok()?;
    Some(email.to_string())
}
