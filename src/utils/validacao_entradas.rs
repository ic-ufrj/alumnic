use regex::Regex;

pub fn processar_dre(dre: &str) -> Option<String> {
    let re = Regex::new(r"^\s*(\d{9})\s*$").unwrap();

    re.captures(dre).map(|caps| format!("{}", &caps[1]))
}

pub fn validar_data_emissao(data: &str) -> Option<String> {
    // Strings do tipo "1/1/2025", "1/1/25", "01/01/2025", etc.
    let re1 = Regex::new(r"^\s*(\d{1,2})\s*/\s*(\d{1,2})\s*/\s*(\d{1,4})\s*$")
        .unwrap();
    // Strings do tipo "01012025", "0101 25", etc.
    let re2 = Regex::new(r"^\s*(\d{2})\s*(\d{2})\s*(\d{1,4})\s*$").unwrap();

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
