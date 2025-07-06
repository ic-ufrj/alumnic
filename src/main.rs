use alumnic::portal_ufrj::{consulta, ConsultaErro};
use chrono::{DateTime, Local};

fn main() -> Result<(), ConsultaErro> {
    let date: DateTime<Local> = serde_json::from_str("\"2025-mes-diaThora:minuto:00-03:00\"").unwrap();

    println!("{:?}", consulta("DRE", date, "XXXX.XXXX.XXXX.XXXX.XXXX.XXXX.XXXX.XXXX")?);

    Ok(())
}

