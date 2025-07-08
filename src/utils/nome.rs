use itertools::Itertools;
use std::str::FromStr;
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Error)]
pub enum NomeErro {
    #[error("O nome possui caracteres desconhecidos")]
    CaracterEstranho,

    #[error("O nome não tem o mínimo de dois nomes")]
    NomeCurto,
}

#[derive(Debug, Clone)]
pub struct Nome(Vec<String>);

impl Nome {
    /// Essa função gera uma lista de nomes de usuário para um nome. Ele gera
    /// todas as possibilidades de sobrenomes inteiros ou iniciais que têm
    /// menos de 20 caracteres
    ///
    /// # Exemplos
    ///
    ///     use alumnic::utils::nome::Nome;
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
        // Isso gera um iterador com os elementos contando em binário, ou seja,
        // algo como isso:
        // [false, false]
        // [false, true]
        // [true, false]
        // [true, true]
        // Isso é usado para testar as possibilidades de abertura dos
        // sobrenomes, false representa somente a primeira letra enquanto true
        // representa o nome inteiro.
        let contagem = std::iter::repeat([false, true])
            .take(self.0.len() - 1)
            .multi_cartesian_product();

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

        contagem
            .map(|m| expansao_sobrenomica(m, &self.0))
            .filter(|u| u.len() < 20)
    }
}

impl FromStr for Nome {
    type Err = NomeErro;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let string_sanitizada = s
            // Substitui os cedilha por C
            .replace('ç', "C")
            .replace('Ç', "C")
            // Separa os acentos dos caracteres
            .nfd()
            // Junto com isso, ele transforma Á em A, etc
            .filter(|x| x.is_ascii())
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
