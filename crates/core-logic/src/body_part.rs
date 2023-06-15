use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BodyPart {
    Head,
    Torso,
    LeftArm,
    RightArm,
    LeftHand,
    RightHand,
    LeftLeg,
    RightLeg,
    LeftFoot,
    RightFoot,
}

impl Display for BodyPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            BodyPart::Head => "head",
            BodyPart::Torso => "torso",
            BodyPart::LeftArm => "left arm",
            BodyPart::RightArm => "right arm",
            BodyPart::LeftHand => "left hand",
            BodyPart::RightHand => "right hand",
            BodyPart::LeftLeg => "left leg",
            BodyPart::RightLeg => "right leg",
            BodyPart::LeftFoot => "left foot",
            BodyPart::RightFoot => "right foot",
        };

        string.fmt(f)
    }
}
