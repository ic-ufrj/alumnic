use regex::Regex;

pub fn validar_dre(dre: &str) -> bool {
    let re = Regex::new(r"^\d{9}$").unwrap();
    re.is_match(dre)
}

pub fn validar_data_emissao(data: &str) -> bool {
    let re = Regex::new(r"\d{2}/\d{2}/\d{4}").unwrap();
    re.is_match(data)
}
