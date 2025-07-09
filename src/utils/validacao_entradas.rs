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
