//! What a room is used for, and what every calculation takes from that:
//! the temperature it is heated to (θint, STN EN 12831-1), the least air
//! it needs (changes an hour or an extract flow), whether air is taken
//! out of it (the air balance), what it gains inside in summer (the
//! cooling load) and whether its floor may run warmer (underfloor
//! heating's edge and wet zones). Heat loss, cooling load, the air balance
//! and underfloor heating all read the one table, so a room typed "WC"
//! is a WC in each of them.
//!
//! A room's type is found in its occupancy, else its name, by the words
//! in them (Slovak, Czech or English): the occupancy can be picked from
//! [`RoomType::ALL`] or typed freely.

/// What a room is used for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RoomType {
    Living,
    Bedroom,
    Children,
    Dining,
    Kitchen,
    Bathroom,
    Toilet,
    Hall,
    Staircase,
    Wardrobe,
    Pantry,
    Laundry,
    Utility,
    Garage,
    Office,
    Ward,
    /// Anything else: heated as a living room, aired at 0.5 an hour.
    Other,
}

/// What a room's summer gains inside are, for the cooling load.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gains {
    Home,
    Kitchen,
    Office,
    /// A bathroom, a corridor, a store: not cooled.
    Service,
}

/// The values a room type gives the calculations.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoomNeeds {
    /// θint, °C.
    pub temp: f64,
    /// Least air changes, 1/h.
    pub ach: Option<f64>,
    /// Least air flow taken out, m³/h.
    pub flow: Option<f64>,
    /// Air is taken out of it (bathroom, WC, kitchen, stores…).
    pub exhaust: bool,
    pub gains: Gains,
    /// Wet room: a floor heated to 33 °C (STN EN 1264), not 29.
    pub wet: bool,
}

impl RoomType {
    pub const ALL: [RoomType; 17] = [
        RoomType::Living,
        RoomType::Bedroom,
        RoomType::Children,
        RoomType::Dining,
        RoomType::Kitchen,
        RoomType::Bathroom,
        RoomType::Toilet,
        RoomType::Hall,
        RoomType::Staircase,
        RoomType::Wardrobe,
        RoomType::Pantry,
        RoomType::Laundry,
        RoomType::Utility,
        RoomType::Garage,
        RoomType::Office,
        RoomType::Ward,
        RoomType::Other,
    ];

    /// Its name, in English: the app translates it, and the translation
    /// is what an occupancy picked from the list reads.
    pub fn label(self) -> &'static str {
        match self {
            RoomType::Living => "Living room",
            RoomType::Bedroom => "Bedroom",
            RoomType::Children => "Children's room",
            RoomType::Dining => "Dining room",
            RoomType::Kitchen => "Kitchen",
            RoomType::Bathroom => "Bathroom",
            RoomType::Toilet => "WC",
            RoomType::Hall => "Hall, corridor",
            RoomType::Staircase => "Staircase",
            RoomType::Wardrobe => "Walk-in wardrobe",
            RoomType::Pantry => "Pantry, storeroom",
            RoomType::Laundry => "Laundry",
            RoomType::Utility => "Technical room, boiler room",
            RoomType::Garage => "Garage",
            RoomType::Office => "Office",
            RoomType::Ward => "Ward, surgery",
            RoomType::Other => "Other",
        }
    }

    /// The words that tell it, lower case, in the order they are tried
    /// (a bathroom with a WC is a bathroom).
    fn words(self) -> &'static [&'static str] {
        match self {
            RoomType::Bathroom => &["kúpeľ", "kupel", "koupel", "bath", "sprch", "shower"],
            RoomType::Toilet => &["wc", "toalet", "toilet", "záchod", "zachod"],
            RoomType::Kitchen => &["kuchy", "kuchyn", "kitchen", "kitchenette"],
            RoomType::Garage => &["garáž", "garaz", "garage"],
            RoomType::Utility => &["techn", "kotol", "kotel", "plant", "boiler", "strojov"],
            RoomType::Laundry => &["práč", "prádel", "pradel", "laundry", "utility room"],
            RoomType::Wardrobe => &["šatn", "satn", "šatník", "wardrobe"],
            RoomType::Pantry => &[
                "špajz", "spajz", "spíž", "spiz", "komor", "pantry", "store", "sklad",
            ],
            RoomType::Staircase => &["schod", "stair", "landing"],
            RoomType::Hall => &[
                "chodb", "hala", "hall", "corridor", "zádver", "zadver", "predsie", "předsí",
                "lobby", "vstup", "entrance",
            ],
            RoomType::Ward => &["lôžk", "ordin", "ošetr", "ward", "surgery"],
            RoomType::Office => &[
                "kancel", "office", "pracov", "study", "učeb", "class", "zasad", "meeting",
            ],
            RoomType::Children => &["detsk", "dětsk", "child", "nursery"],
            RoomType::Bedroom => &["spál", "spal", "ložnic", "loznic", "bedroom"],
            RoomType::Dining => &["jedál", "jedal", "jídel", "jidel", "dining"],
            RoomType::Living => &["obýv", "obyv", "living", "izba", "pokoj", "room"],
            RoomType::Other => &[],
        }
    }

    /// The type a room's occupancy and name tell: the occupancy first.
    pub fn of(occupancy: &str, name: &str) -> RoomType {
        Self::find(occupancy).unwrap_or_else(|| Self::find(name).unwrap_or(RoomType::Other))
    }

    /// The type one text tells, if any.
    pub fn find(text: &str) -> Option<RoomType> {
        let t = format!("{} ", text.trim().to_lowercase());
        if t.trim().is_empty() {
            return None;
        }
        // the wet and service rooms before the rest: "Kúpeľňa a WC",
        // "Kuchyňa s jedálňou", "Chodba pri spálni"
        const ORDER: [RoomType; 16] = [
            RoomType::Bathroom,
            RoomType::Toilet,
            RoomType::Kitchen,
            RoomType::Garage,
            RoomType::Utility,
            RoomType::Laundry,
            RoomType::Wardrobe,
            RoomType::Pantry,
            RoomType::Staircase,
            RoomType::Hall,
            RoomType::Ward,
            RoomType::Office,
            RoomType::Children,
            RoomType::Bedroom,
            RoomType::Dining,
            RoomType::Living,
        ];
        ORDER
            .into_iter()
            .find(|r| r.words().iter().any(|w| t.contains(w)))
    }

    /// What the calculations take from it.
    ///
    /// θint: STN EN 12831-1:2019 table B.14 gives a dwelling's rooms and
    /// an office 20 °C, a bathroom 24 °C; it has no WC, hall, stairs,
    /// store, garage or technical room, so those keep the values the heat
    /// loss used before (WC 20, hall, stairs and stores 15, technical
    /// room 15, garage 5, ward 22 °C) until a national value is given.
    /// Least air changes: its table B.7, 0.5/h in rooms people stay in
    /// and in kitchens, bathrooms and WCs, 0 in secondary and inner
    /// rooms. Extract flows for dwellings (kitchen 100, bathroom 90, WC
    /// 50 m³/h) and an office's 25 m³/h are not from that norm; the
    /// larger of the two is taken.
    pub fn needs(self) -> RoomNeeds {
        let n =
            |temp: f64, ach: Option<f64>, flow: Option<f64>, exhaust: bool, gains, wet| RoomNeeds {
                temp,
                ach,
                flow,
                exhaust,
                gains,
                wet,
            };
        use Gains::*;
        match self {
            RoomType::Living
            | RoomType::Bedroom
            | RoomType::Children
            | RoomType::Dining
            | RoomType::Other => n(20.0, Some(0.5), None, false, Home, false),
            RoomType::Kitchen => n(20.0, Some(0.5), Some(100.0), true, Kitchen, false),
            RoomType::Bathroom => n(24.0, Some(0.5), Some(90.0), true, Service, true),
            RoomType::Toilet => n(20.0, Some(0.5), Some(50.0), true, Service, true),
            RoomType::Hall | RoomType::Staircase => n(15.0, Some(0.0), None, false, Service, false),
            RoomType::Wardrobe | RoomType::Pantry => n(15.0, Some(0.0), None, true, Service, false),
            RoomType::Laundry => n(20.0, Some(0.5), Some(50.0), true, Service, true),
            RoomType::Utility => n(15.0, Some(0.0), None, true, Service, false),
            RoomType::Garage => n(5.0, Some(0.0), None, true, Service, false),
            RoomType::Office => n(20.0, Some(0.5), Some(25.0), false, Office, false),
            RoomType::Ward => n(22.0, Some(0.5), None, false, Home, false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rooms_are_told_by_their_occupancy_then_their_name() {
        assert_eq!(RoomType::of("", "WC"), RoomType::Toilet);
        assert_eq!(RoomType::of("WC", "1.04"), RoomType::Toilet);
        assert_eq!(RoomType::of("", "Kúpeľňa a WC"), RoomType::Bathroom);
        assert_eq!(RoomType::of("", "Koupelna"), RoomType::Bathroom);
        assert_eq!(RoomType::of("", "Obývacia izba"), RoomType::Living);
        assert_eq!(RoomType::of("", "Detská izba"), RoomType::Children);
        assert_eq!(RoomType::of("", "Pracovňa"), RoomType::Office);
        assert_eq!(RoomType::of("", "Práčovňa"), RoomType::Laundry);
        assert_eq!(RoomType::of("", "Kotolňa"), RoomType::Utility);
        assert_eq!(RoomType::of("", "Zádverie"), RoomType::Hall);
        assert_eq!(RoomType::of("", "Schodisko"), RoomType::Staircase);
        assert_eq!(RoomType::of("Garáž", "Garage 1"), RoomType::Garage);
        assert_eq!(RoomType::of("", "1.07"), RoomType::Other);
        // the occupancy wins over the name
        assert_eq!(RoomType::of("Bathroom", "Room 3"), RoomType::Bathroom);
        // every type's English name finds it again
        for t in RoomType::ALL {
            if t != RoomType::Other {
                assert_eq!(RoomType::find(t.label()), Some(t), "{}", t.label());
            }
        }
        let wc = RoomType::Toilet.needs();
        assert!(wc.exhaust && wc.wet && wc.flow == Some(50.0) && wc.gains == Gains::Service);
        assert_eq!(RoomType::Bathroom.needs().temp, 24.0);
    }
}
