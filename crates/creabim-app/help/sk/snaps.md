# Prichytávanie

Pri kreslení alebo presúvaní sa ukazovateľ **prichytáva** k presným bodom modelu, takže steny sa stretnú a veci sa zarovnajú bez hádania. Značka ukazuje druh prichytenia a popis ho pomenuje, napríklad *Koncový bod* alebo *Stred*.

## Druhy prichytenia {#kinds-of-snaps}

Zapnuté druhy vyberiete v zozname vedľa **Prichytávanie** v stavovom riadku. Každý možno zaškrtnúť samostatne; *Zapnúť všetko* a *Vypnúť všetko* nastavia všetky.

- **Koncový bod**: konce a rohy stien, dosiek, čiar.
- **Stred**: stred hrany.
- **Priesečník**: miesto, kde sa pretínajú dve hrany.
- **Kolmica**: kolmo na hranu od posledného bodu.
- **Najbližší**: ľubovoľný bod na hrane.
- **Stred kružnice** a **Kvadrant**: kružnice a oblúky.
- **Osi a čiary podlaží**.
- **Pomocné čiary**: [pomocné čiary](guides).
- **Sledovanie osí**: v línii s posledným bodom pozdĺž X alebo Y.
- **Zarovnanie s bodmi tvaru**: vodorovne alebo zvislo s rohmi, na ktoré ste klikli, s čiarkovanými čiarami.
- **Delenie**: každá hrana na rovnaké diely: 3, 4, 5, 6, 8 alebo 10 (*Delenie* v zozname).
- **Čiary podkladu**: konce, stredy a čiary [podkladu](trace-reference) v pôdorysoch, rezoch aj 3D.
- **Na ploche** (3D): úchop posúvaný po čiare sa zastaví na ploche pod ukazovateľom, s malým modrým štvorcom.

Prichytávanie je predvolene **voľné**: body a ťahania idú presne tam, kde je ukazovateľ, a prichytenia ich zachytia, keď sa ukazovateľ niečomu priblíži.

## Vypnutie prichytávania {#switching-snaps-off}

- Kliknite na **Prichytávanie** v stavovom riadku alebo stlačte `F3` a vypnete či zapnete celé prichytávanie.
- Podržaním `Alt` pri kliknutí prichytávanie pre toto kliknutie potlačíte.

## Prichytávanie k mriežke a značky vzdialenosti {#grid-snap-and-distance-marks}

**Prichytávanie k mriežke** (`Alt+S` alebo jeho ponuka v stavovom riadku) je po každom spustení aplikácie vypnuté. Keď je zapnuté, body padnú na jeho mriežku (10, 50 alebo 100 mm alebo ľubovoľný zadaný rozostup) a dĺžky pozdĺž osí a pomocných čiar sa na ňu zaokrúhlia. *Značky vzdialenosti* v tej istej ponuke zobrazujú pri ťahaní slabé značky v každom rozostupe a podržia ťahanie, keď sa k niektorej priblíži na 10 px. Pozri [Úchopy](handles).

## Prichytávanie k iným podlažiam {#snapping-to-other-levels}

[Podklad](levels#underlay) pôdorysu zobrazuje slabo iné podlažie a k jeho čiaram sa možno prichytiť.
