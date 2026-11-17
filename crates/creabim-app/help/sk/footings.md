# Základy

**Základ** je založenie pod budovou. **Základový pás** vedie pod stenou. **Základová pätka** je pod stĺpom alebo stojí samostatne.

## Umiestnenie základov {#placing-footings}

1. Zvoľte nástroj **Základ** (vo *Vytvoriť › Stavebné prvky*, vedľa Zábradlia).
2. Vo voľbách nástroja zvoľte **Pás** alebo **Pätka**, potom nastavte *Šírku* (pri pätke *Rozmer*) a *Hrúbku*. Môžete zvoliť aj **Typ** základu.
3. Kliknutím umiestnite:
   - **Pás:** kliknite na stenu a pás sa dá pod ňu. Samostatný pás nakreslíte kliknutím na jeho dva konce inde.
   - **Pätka:** kliknite na stĺp a pätka sa dá pod neho, alebo kliknite kamkoľvek a umiestnite samostatnú.

Ďalšími kliknutiami na steny (alebo stĺpy) dáte každému jeho základ: nástroj zostane zapnutý, kým nestlačíte `Esc`.

Základy pod mnoho prvkov naraz:

- **Ťahajte obdĺžnik** nástrojom Základ, ako pri Výbere. Každá stena, ktorej sa dotkne, dostane pás (pri zvolenom *Páse*), alebo každý stĺp pätku (pri *Pätke*), všetko jedným krokom späť.
- Alebo najprv vyberte steny a stĺpy, potom kliknite na **Pod N vybranými** vo voľbách nástroja, alebo pravým tlačidlom zvoľte **Základy pod výberom**. Každá stena dostane pás a každý stĺp pätku.

Čo už základ má, sa preskočí, a závesové steny ho nikdy nedostanú.

## Základy sledujú to, čo nesú {#footings-follow-what-they-carry}

Základ umiestnený pod stenou alebo stĺpom je ním **hostený**:

- Pás vedie po dĺžke steny. Kde sú steny napojené, ich pásy sú v rohoch na pokos a v T-spojoch orezané, rovnako ako steny. Kde stena končí voľne, pás ju presahuje o polovicu rozšírenia.
- Pätka zostáva vycentrovaná pod stĺpom a otáča sa s ním.
- Keď sa stena alebo stĺp posunie, základ sa posunie s ním. Keď sa odstráni, odstráni sa aj jeho základ.
- Jeho hĺbka sa meria od spodku hostiteľa. Ak stena alebo stĺp stojí na podlaží vyššie, základ ide hore s ním.

Hostený základ osamostatníte kliknutím na **Uvoľniť** vo *Vlastnostiach*. Zostane na mieste, ale hostiteľa už nesleduje.

## Rozmer a hĺbka {#size-and-depth}

*Vlastnosti* ukazujú *Šírku* základu, *Dĺžku* pätky, jej *Hrúbku* a **Vrch pod**: ako hlboko je jeho vrch pod spodkom steny alebo stĺpa. Pri samostatnom základe sa meria od jeho podlažia. Posledný riadok uvádza výškové úrovne vrchu a spodku.

Samostatný základ má aj *Podlažie* a polohu, pätka aj *Otočenie*.

**Typy** základov nesú druh (pás alebo pätka), rozmer, hrúbku a materiál. Dodávajú sa štyri základné: *Základový pás 600 × 300*, *Základový pás 900 × 400*, *Pätka 1200 × 1200 × 400* a *Pätka 1800 × 1800 × 500*. Pozri [Typy](types).

## Úchyty {#handles}

Vyberte základ a uvidíte jeho úchyty. Fungujú v pôdorysoch, rezoch, pohľadoch aj v 3D:

- **Šírka** a pri pätke **Dĺžka**: ťahajte bok.
- **Vrch** a **Spodok**: ťahajte hore alebo dole v reze a zmeníte hĺbku alebo hrúbku.
- **Konce** a **Stred**: posúvajú samostatný základ. Hostený základ sa namiesto toho posúva so svojím hostiteľom.

[Režim úprav](edit-mode) ukazuje rovnaké úchyty. [Posun](move) a *Otočenie* fungujú na samostatných základoch.

## Vo výkresoch a výkazoch {#in-drawings-and-schedules}

Základy sú pod podlahou, preto ich pôdorysy kreslia **čiarkovane**. Pôdorys prerezaný dostatočne nízko, aby prechádzal základom, ho ukáže prerezaný. Rezy a pohľady ich kreslia prerezané alebo v pohľade a 3D ich ukazuje v betónovej sivej.

*Súbor › Nový výkaz › Stĺpy a nosníky* vypíše aj základy s ich typom, podlažím a rozmerom. Kategóriou *Základy* vo [Viditeľnosti](visibility) ich v pohľade skryjete.

## IFC

Základy sa exportujú ako **IfcFooting**, s preddefinovaným typom *STRIP_FOOTING* alebo *PAD_FOOTING* a ich typy ako **IfcFootingType**. Sada vlastností *CreaBIM_Footing* uchováva rozmer, hĺbku a hostiteľa, takže pri opätovnom otvorení súboru sa základ vráti ako základ, stále pod svojou stenou či stĺpom. Pozri [IFC](ifc).
