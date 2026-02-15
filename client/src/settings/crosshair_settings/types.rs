use crate::weapons::crosshair::CrosshairType;

pub const CROSSHAIR_TYPES: &[CrosshairType] = &[
    CrosshairType::Cross,
    CrosshairType::FourCurves,
    CrosshairType::Chevron,
    CrosshairType::Circle,
    CrosshairType::Square,
    CrosshairType::Triangle,
    CrosshairType::AngleBrackets,
];

pub fn crosshair_type_name(ct: CrosshairType) -> &'static str {
    match ct {
        CrosshairType::Cross => "Cross",
        CrosshairType::FourCurves => "4-Curves",
        CrosshairType::Chevron => "Chevron",
        CrosshairType::Circle => "Circle",
        CrosshairType::Square => "Square",
        CrosshairType::Triangle => "Triangle",
        CrosshairType::AngleBrackets => "Angle Brackets",
    }
}
