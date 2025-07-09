pub fn validar_dre(dre: &str) -> bool {
    dre.len() == 9 && dre.chars().all(|c| c.is_ascii_digit())
}

pub fn validar_data_emissao(data: &str) -> bool {
    data.is_ascii()
        && data.len() == 5
        && &data[2..=2] == "/"
        && data[0..=1].chars().all(|c| c.is_ascii_digit())
        && data[3..=4].chars().all(|c| c.is_ascii_digit())
}
