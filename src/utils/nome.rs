//! Esse módulo implementa funções para lidar com nomes, ele é capaz de fazer
//! uma comparação mais bruta entre nomes (por exemplo, "JOSE LIMA SILVA" é
//! considerado igual a "José Lima da Silva") e também possui a função
//! [`Nome::usernames`], que é um iterador de nomes de usuário válidos para usar
//! nos sistemas do Instituto.

use itertools::Itertools;
use std::str::FromStr;
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

/// Um erro ao tentar converter uma string para um [Nome]. Ocorre quando o nome
/// não é considerado válido.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum NomeErro {
    /// O nome tem caracteres que não são letras, com ou sem acentos, cedilhas
    /// ou espaços.
    #[error("O nome possui caracteres desconhecidos")]
    CaracterEstranho,

    /// O nome não possui o mínimo de 2 palavras, sem contar "de", "da", etc.
    #[error("O nome não tem o mínimo de dois nomes")]
    NomeCurto,
}

/// Representação bruta e simplificada de um nome. Letras com acentos têm seus
/// acentos ignorados e cedilhas são substituídas por "c"s. Além disso, as
/// palavras "de", "do", "da", "dos" e "das" são removidas. Isso é útil para
/// gerar os uids para o LDAP e para comparar strings com nomes.
///
/// # Examples
///
///     # use alumnic::utils::nome::Nome;
///     let nome1: Nome = "JOSE FELIPE ARAUJO".parse().unwrap();
///     let nome2: Nome = "José Felipe de Araújo".parse().unwrap();
///
///     assert_eq!(nome1, nome2);
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nome(Vec<String>);

impl Nome {
    /// Essa função gera uma lista de nomes de usuário para um nome. Ele gera
    /// todas as possibilidades de sobrenomes inteiros ou iniciais que têm
    /// menos de 20 caracteres.
    ///
    /// # Examples
    ///
    ///     # use alumnic::utils::nome::Nome;
    ///
    ///     let nome: Nome = "ARTHUR BACCI DE OLIVEIRA".parse().unwrap();
    ///     assert_eq!(
    ///         nome.usernames().collect::<Vec<String>>(),
    ///         vec![
    ///             "arthurbo",
    ///             "arthurboliveira",
    ///             "arthurbaccio",
    ///             "arthurbaccioliveira",
    ///         ],
    ///     );
    ///
    pub fn usernames(&self) -> impl Iterator<Item = String> {
        fn expansao_sobrenomica(mask: Vec<bool>, names: &[String]) -> String {
            let sobrenomes_expandidos =
                mask.into_iter().enumerate().map(|(i, e)| {
                    if e {
                        names[i + 1].clone()
                    } else {
                        names[i + 1][0..=0].to_string()
                    }
                });

            std::iter::once(names[0].clone())
                .chain(sobrenomes_expandidos)
                .collect()
        }

        // Isso gera um iterador com os elementos contando em binário, ou seja,
        // algo como isso:
        // [false, false]
        // [false, true]
        // [true, false]
        // [true, true]
        // Isso é usado para testar as possibilidades de abertura dos
        // sobrenomes, false representa somente a primeira letra enquanto true
        // representa o nome inteiro.
        let contagem = std::iter::repeat_n([false, true], self.0.len() - 1)
            .multi_cartesian_product();


        contagem
            .map(|m| expansao_sobrenomica(m, &self.0))
            .filter(|u| u.len() < 20)
    }
}

impl FromStr for Nome {
    type Err = NomeErro;

    /// Converte uma string para um [Nome].
    ///
    /// # Examples
    ///
    ///     # use alumnic::utils::nome::{Nome, NomeErro};
    ///     # use std::str::FromStr;
    ///     let nome1 = Nome::from_str("José");
    ///     let nome2 = "Carlos Pereira".parse::<Nome>();
    ///     let nome3 = "Carlos 71".parse::<Nome>();
    ///
    ///     assert_eq!(nome1, Err(NomeErro::NomeCurto));
    ///     assert!(nome2.is_ok());
    ///     assert_eq!(nome3, Err(NomeErro::CaracterEstranho));
    ///
    /// # Errors
    ///
    /// - Retorna erro se o nome possuir um caractere que não seja uma letra,
    ///   com ou sem acento, um cedilha ou um espaço; e
    /// - Retorna erro se o nome não possuir o mínimo de duas palavras, sem
    ///   contar "de", "do", etc.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let string_sanitizada = s
            // Substitui os cedilha por C
            .replace(['ç', 'Ç'], "C")
            // Separa os acentos dos caracteres
            .nfd()
            // Junto com isso, ele transforma Á em A, etc
            .filter(char::is_ascii)
            // Agora ele faz tudo ficar minúsculo
            .map(|x| x.to_ascii_lowercase())
            // Se existir um caractere que não seja uma letra minúscula ou
            // um espaço, ele é um erro
            .map(|x| {
                (x.is_ascii_lowercase() || x == ' ')
                    .then_some(x)
                    .ok_or(NomeErro::CaracterEstranho)
            })
            // Transforma o Iterator<Result<char, NomeErro>> em
            // Result<String, NomeErro> e, por fim, delega esse erro para a
            // função
            .collect::<Result<String, Self::Err>>()?;

        let mut v: Vec<String> = string_sanitizada
            .split_whitespace()
            .filter(|x| !x.is_empty())
            .filter(|x| !["de", "do", "da", "dos", "das"].contains(x))
            .map(str::to_string)
            .collect();

        // Cortar nomes gigantes, talvez seja melhor retornar um erro, mas pode
        // ser que existam pessoas com mais de 10 nomes, talvez
        v.truncate(10);

        // É importante que não se crie nomes sem sobrenome, mas, caso permita,
        // é necessário modificar a geração de usernamees para não assumir que
        // os nomes sempre têm ao menos um sobrenome
        if v.len() > 1 {
            Ok(Nome(v))
        } else {
            Err(NomeErro::NomeCurto)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn testar_usernames() {
        assert_eq!(
            Nome::from_str("JOÃO CARLOS PEREIRA DA SILVA")
                .unwrap()
                .usernames()
                .collect::<Vec<String>>(),
            vec![
                "joaocps",
                "joaocpsilva",
                "joaocpereiras",
                "joaocpereirasilva",
                "joaocarlosps",
                "joaocarlospsilva",
                "joaocarlospereiras",
            ],
        );
    }
}
