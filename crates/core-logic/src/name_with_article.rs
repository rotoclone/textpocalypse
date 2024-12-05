use std::fmt::Display;

/// The name of something including the article to use with it.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct NameWithArticle {
    /// The indefinite article
    pub article: IndefiniteArticle,
    /// THe name
    pub name: String,
}

impl NameWithArticle {
    /// Creates a `NameWithArticle` with the article "a"
    pub fn a<T: Into<String>>(name: T) -> NameWithArticle {
        NameWithArticle {
            article: IndefiniteArticle::A,
            name: name.into(),
        }
    }

    /// Creates a `NameWithArticle` with the article "an"
    pub fn an<T: Into<String>>(name: T) -> NameWithArticle {
        NameWithArticle {
            article: IndefiniteArticle::An,
            name: name.into(),
        }
    }
}

impl Display for NameWithArticle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        format!("{} {}", self.article, self.name).fmt(f)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndefiniteArticle {
    A,
    An,
}

impl Display for IndefiniteArticle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            IndefiniteArticle::A => "a",
            IndefiniteArticle::An => "an",
        };

        string.fmt(f)
    }
}
