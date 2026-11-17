# Jednotky a formáty čísel

## Jednotky projektu {#project-units}

*Súbor › Jednotky projektu…* alebo ponuka **Jednotky** v stavovom riadku nastavuje, ako sa v tomto projekte zobrazujú a zadávajú dĺžky:

- **Metrické**: milimetre, centimetre alebo metre s počtom desatinných miest;
- **Anglosaské**: stopy a zlomkové palce, desatinné stopy alebo palce so zaokrúhlením.

Náhľad zobrazuje výsledok. Každé pole, údaj, kóta, značka, výkaz a list sa okamžite prispôsobí. Voľba sa ukladá s projektom a dá sa vrátiť.

## Formáty čísel {#number-formats}

Každý druh popisu môže zapisovať čísla po svojom: **kóty**, **značky podlaží**, **výškové kóty**, **značky** a **výkazy**. Ponuka **Jednotky** v stavovom riadku vypisuje každý druh s ukážkou, ako teraz zapisuje; kliknutím na jeden alebo na *Formáty čísel…* otvoríte okno **Formáty čísel**. Najprv je *Projekt*, potom každý druh.

Pri druhu začína každé nastavenie ako *Podľa projektu* a dá sa nastaviť samostatne:

| Nastavenie | Možnosti |
| --- | --- |
| **Jednotka** | ľubovoľná jednotka dĺžky, metrická alebo anglosaská |
| **Desatinné miesta** (alebo **Zaokrúhlenie**) | 0 až 4 desatinné miesta; stopy a palce sa zaokrúhľujú na zlomok |
| **Značka jednotky** | zobraziť, skryť, alebo ako ju zapisuje jednotka |
| **Nuly** | úvodná nula (`0,450` alebo `,450`) a koncové nuly (`3,000` alebo `3`) |
| **Znamienko** (podlažia) | `+3` a `±0`, alebo samotné číslo |

*Vyzerá ako* zobrazuje ukážku počas zmien. *Obnoviť podľa projektu* vráti všetky polia. Formáty čísel sa ukladajú v [balíku štýlov](graphic-styles), takže ich prenáša súbor grafických štandardov.

Jedna kóta môže vo *Vlastnostiach* prepísať formát svojho druhu. Pozri [Kóty](dimensions).

## Zadávanie dĺžok {#typing-lengths}

Každé pole dĺžky prijme číslo v jednotkách projektu alebo v ľubovoľnej vypísanej jednotke:

| Zadáte | V metrickom projekte (mm) |
| --- | --- |
| `1200` | 1200 mm |
| `1.2 m` | 1200 mm |
| `120 cm` | 1200 mm |
| `4'` | 1219 mm |
| `3'-6 1/2"` | 1080 mm |

V projekte v stopách a palcoch sa `3 6` načíta ako 3′-6″ a `42"` ako 3′-6″.

> **Poznámka:** Formáty papiera, veľkosti textu a hrúbky čiar na listoch zostávajú v milimetroch, ako je pri výkresoch obvyklé.
