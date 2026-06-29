//! Esse módulo implementa funções para lidar com nomes, ele é capaz de fazer
//! uma comparação mais bruta entre nomes (por exemplo, "JOSE LIMA SILVA" é
//! considerado igual a "José Lima da Silva") e também possui a função
//! [`Nome::usernames`], que é um iterador de nomes de usuário válidos para usar
//! nos sistemas do Instituto.

use itertools::Itertools;
use std::str::FromStr;
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;
use derive_more::Display;


/// Um erro ao tentar converter uma string para um [Nome]. Ocorre quando o nome
/// não é considerado válido.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum NomeErro {
    /// O nome tem caracteres que não são letras, com ou sem acentos, cedilhas
    /// ou espaços.
    #[error("O nome possui caracteres desconhecidos")]
    CaracterEstranho,
}

/// Essa estrutura guarda um nome completo devidamente sanitizado.
///
/// Ao converter um [Nome] para uma [String] o resultado é o nome completo
/// separado por espaços devidamente capitalizado. As únicas palavras que não
/// são capitalizadas são "de", "do", "da", "dos", "das" e "e".
///
/// [Nome]s "aproximadamente iguais" são considerados iguais. Diferenças na
/// acentuação e na existência ou não das palavras "de", "do", etc. são
/// ignoradas.
///
/// [Nome]s só podem ser formados por letras do alfabeto latino, letras
/// acentuadas, cedilhas e espaços.
///
/// # Examples
///
/// ```
/// use alumnic::utils::nome::{Nome, NomeErro};
///
/// let nome1: Nome = "ALEXANDRE COELHO GONCALVES DA COSTA".parse().unwrap();
/// let nome2: Nome = "Alexandre Coelho Gonçalves Da Costa".parse().unwrap();
///
/// // Acentos inexistentes na criação do [Nome] obviamente não podem ser
/// // recuperados ao formatar.
/// assert_eq!(
///     nome1.to_string(),
///     "Alexandre Coelho Goncalves da Costa",
/// );
/// assert_eq!(
///     nome2.to_string(),
///     "Alexandre Coelho Gonçalves da Costa",
/// );
///
/// // Os dois nomes são iguais, apesar da diferença do acento.
/// assert_eq!(nome1, nome2);
///
/// // Caracteres inválidos não são aceitos
/// assert_eq!(
///     "lajdf[q] 19293".parse::<Nome>(),
///     Err(NomeErro::CaracterEstranho),
/// )
/// ```
#[derive(Debug, Clone, Display)]
pub struct Nome(String);

impl Nome {
    /// Essa função gera um iterador de nomes de usuários possíveis para um
    /// determinado [Nome]. Esses nomes de usuário são formados a partir de
    /// junções do primeiro nome com combinações de iniciais e sobrenomes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use alumnic::utils::nome::Nome;
    /// let nome1: Nome = "Valter Luiz da Silva".parse().unwrap();
    ///
    /// assert_eq!(
    ///     nome1.usernames().collect::<Vec<String>>(),
    ///     vec![
    ///         "valterls",
    ///         "valterlsilva",
    ///         "valterluizs",
    ///         "valterluizsilva",
    ///     ],
    /// );
    /// ```
    pub fn usernames(&self) -> impl Iterator<Item = String> {
        // Essa função recebe uma máscara e retorna um username gerado a partir
        // dela. Por exemplo: mascara = [false, true, false],
        // names = [Jose, Pereira, Augusto, Silva], então ele vai gerar um nome
        // "josepaugustos". Se fosse [true, false, false], seria
        // "josepereiraas". Vale ressaltar que o primeiro nome sempre aparece
        // inteiro.
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

        let nomes: Vec<String> = sem_acentos_e_minusculo(&self.0)
            .split_whitespace()
            .filter(|x| !["de", "do", "da", "dos", "das", "e"].contains(x))
            .map(str::to_string)
            .collect();

        // Isso gera um iterador com os elementos contando em binário, ou seja,
        // algo como isso:
        // [false, false]
        // [false, true]
        // [true, false]
        // [true, true]
        // Isso é usado para testar as possibilidades de abertura dos
        // sobrenomes, false representa somente a primeira letra enquanto true
        // representa o nome inteiro.
        let contagem = std::iter::repeat_n([false, true], nomes.len() - 1)
            .multi_cartesian_product();

        contagem
            .map(move |m| expansao_sobrenomica(m, &nomes))
            .filter(|u| u.len() < 20)
    }
}

impl PartialEq for Nome {
    /// Compara se dois [Nome]s são iguais. Nessa comparação, diferenças quanto
    /// à presença das palavras "de", "do", "da", "dos", "das" e "e" são
    /// ignoradas. Diferenças de acentuação das letras também são ignoradas.
    ///
    /// # Examples
    ///
    /// ```
    /// # use alumnic::utils::nome::Nome;
    /// assert_eq!(
    ///     "CarlOS JosÉ DA costa".parse::<Nome>().unwrap(),
    ///     "CARLOS JOSE COSTA".parse::<Nome>().unwrap(),
    /// );
    /// ```
    fn eq(&self, other: &Self) -> bool {
        let s = sem_acentos_e_minusculo(&self.0);
        let o = sem_acentos_e_minusculo(&other.0);
        let a = s
            .split_whitespace()
            .filter(|x| !["de", "do", "da", "dos", "das", "e"].contains(x));
        let b = o
            .split_whitespace()
            .filter(|x| !["de", "do", "da", "dos", "das", "e"].contains(x));

        a.eq(b)
    }
}

impl Eq for Nome {}

impl FromStr for Nome {
    type Err = NomeErro;

    /// Cria um [Nome] a partir de uma string. Somente letras do alfabeto
    /// latino, com ou sem acentos, cedilhas e espaços são aceitos na string.
    ///
    /// # Errors
    ///
    /// - [NomeErro::CaracterEstranho] no caso de um carácter não aceito.
    ///
    /// # Examples
    ///
    /// ```
    /// # use alumnic::utils::nome::{Nome, NomeErro};
    /// assert_eq!(
    ///     "10923 1892381 18283".parse::<Nome>(),
    ///     Err(NomeErro::CaracterEstranho),
    /// );
    /// assert_eq!(
    ///     "dez vinte trinta".parse::<Nome>().unwrap().to_string(),
    ///     "Dez Vinte Trinta",
    /// );
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let sem_caracteres_estranhos = s
            // Substitui os cedilha por C
            .replace(['ç', 'Ç'], "C")
            // Separa os acentos dos caracteres
            .nfd()
            // Remove os acentos separados, transformando Á em A, por exemplo
            .filter(char::is_ascii)
            // Agora ele faz tudo ficar minúsculo
            .map(|x| x.to_ascii_lowercase())
            // Se existir um caractere que não seja uma letra minúscula ou
            // um espaço, ele é um erro
            .all(|x| x.is_ascii_lowercase() || x == ' ');

        if !sem_caracteres_estranhos {
            return Err(NomeErro::CaracterEstranho);
        }

        let nome_limpo = s
            .split_whitespace()
            .filter(|x| !x.is_empty())
            .map(str::to_lowercase)
            // TODO: colocar de do da dos das e em uma variável estática global
            .map(|x| if !["de", "do", "da", "dos", "das", "e"].contains(&x.as_str()) {
                capitalize(&x)
            } else {
                x
            })
            .join(" ");

        Ok(Self(nome_limpo))
    }
}

fn sem_acentos_e_minusculo(a: &str) -> String {
    a
        // Substitui os cedilha por C
        .replace(['ç', 'Ç'], "C")
        // Separa os acentos dos caracteres
        .nfd()
        // Remove os acentos separados, transformando Á em A, por exemplo
        .filter(char::is_ascii)
        // Agora ele faz tudo ficar minúsculo
        .map(|x| x.to_ascii_lowercase())
        .collect()
}

fn capitalize(a: &str) -> String {
    let mut c = a.chars();
    match c.next() {
        Some(fst) => fst.to_uppercase().collect::<String>() + c.as_str(),
        None => "".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn testar_usernames() {
        assert_eq!(
            Nome::from_str("CLÁUDiO de lima Cavalcante")
                .unwrap()
                .usernames()
                .collect::<Vec<String>>(),
            vec![
                "claudiolc",
                "claudiolcavalcante",
                "claudiolimac",
                // Não está incluso porque passa do limite de 19 caracteres
                // "claudiolimacavalcante",
            ],
        );

        assert_eq!(
            Nome::from_str("luiz renato medeiros mota da silva duarte")
                .unwrap()
                .usernames()
                .collect::<Vec<String>>(),
            vec![
                "luizrmmsd",
                "luizrmmsduarte",
                "luizrmmsilvad",
                "luizrmmsilvaduarte",
                "luizrmmotasd",
                "luizrmmotasduarte",
                "luizrmmotasilvad",
                "luizrmedeirosmsd",
                "luizrmedeirosmotasd",
                "luizrenatommsd",
                "luizrenatommsduarte",
                "luizrenatommsilvad",
                "luizrenatommotasd",
            ],
        );
    }

    #[test]
    fn testar_comparacoes() {
        assert_eq!(
            Nome::from_str("CLAUDIO LIMA CAVALCANTE"),
            Nome::from_str("Cláudio de Lima CavalcantE")
        );
        assert_eq!(
            &Nome::from_str("CLÁUDIO DE LIMA CAVALCANTE").unwrap().to_string(),
            "Cláudio de Lima Cavalcante"
        );
        assert_ne!(
            Nome::from_str("CLAUDIO LIMA CAVALCANTE"),
            Nome::from_str("CLAUDIO L CAVALCANTE")
        );
    }
}
