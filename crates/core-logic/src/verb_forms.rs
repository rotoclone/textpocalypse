/// Contains different forms of a verb.
#[derive(Clone)]
pub struct VerbForms {
    /// The second-person form, to follow "You"
    pub second_person: String,
    /// The third-person plural form, to follow e.g. "They"
    pub third_person_plural: String,
    /// The third-person singular form, to follow e.g. "He"/"She"/"It"
    pub third_person_singular: String,
}
