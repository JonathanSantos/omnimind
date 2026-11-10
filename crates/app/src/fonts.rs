//! Fontes embutidas no binário. Todas sob OFL 1.1; licenças em `assets/fonts`.
pub const FONTS: &[&[u8]] = &[
    include_bytes!("../../../assets/fonts/FamiljenGrotesk.ttf"),
    include_bytes!("../../../assets/fonts/FamiljenGrotesk-Italic.ttf"),
    include_bytes!("../../../assets/fonts/SometypeMono.ttf"),
];
