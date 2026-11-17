# Stĺpy a nosníky

## Stĺpy {#columns}

1. Zvoľte nástroj **Stĺp**.
2. Vo voľbách nástroja zvoľte *Štvorcový* alebo *Kruhový* a *Rozmer*, profil z [knižnice](profiles) (I, T, U, vlastný), alebo **Typ** stĺpa.
3. Kliknite tam, kam má ísť každý stĺp.

Stĺp siaha od svojho podlažia po najbližšie vyššie, pokiaľ jeho vrch neviažete na podlažie alebo mu nedáte pevnú výšku vo *Vlastnostiach*. *Vlastnosti* obsahujú aj jeho natočenie a **Profil**: *Vlastné (šírka × hĺbka)*, alebo ľubovoľný profil z [knižnice](profiles), napríklad profil I alebo rúru. Ťahaním stredového úchytu ho posuniete, horným úchytom zmeníte výšku a spodným odsadenie spodku.

## Cez viac podlaží {#over-several-levels}

*Vlastnosti* stĺpa majú štyri riadky, ktoré určujú, kde začína a končí:

- **Spodné podlažie** a **Odsadenie spodku**: podlažie, na ktorom stojí, a ako vysoko nad ním je jeho päta.
- **Horné podlažie** a **Odsadenie vrchu**: podlažie, na ktoré je viazaný jeho vrch, a ako vysoko nad ním vrch je. *Nepripojené* znamená pevnú výšku, alebo po najbližšie podlažie.

Keď je vrch viazaný na podlažie, stĺp sa natiahne pri posune ktoréhokoľvek z podlaží.

Napríklad stĺp nakreslený na Podlaží 5, ktorý má stáť na Podlaží 3 a siahať po Podlažie 5:

1. Vyberte ho a nastavte *Horné podlažie* na Podlažie 5.
2. Nastavte *Spodné podlažie* na Podlažie 3. Vrch zostane na mieste, takže stĺp teraz prechádza Podlažím 4.

Zmena spodného podlažia vždy ponechá vrch na mieste. Stĺp, ktorý siahal po najbližšie podlažie, zostane naň viazaný.

V reze alebo pohľade ťahajte jeho horný alebo spodný úchyt hore či dole. Keď je vrch viazaný na podlažie, ťahanie mení *Odsadenie vrchu*.

Stĺp prechádzajúci podlažím sa kreslí prerezaný aj v pôdoryse tohto podlažia.

Kopírovanie stĺpa na iné podlažia cez **Kopírovať na podlažia…** zachová jeho rozpätie v podlažiach. Pozri [Podlažia](levels#copying-and-moving-to-other-levels).

## Nosníky {#beams}

1. Zvoľte nástroj **Nosník**. Vo voľbách nástroja nastavte *Šírku* a *Výšku*, zvoľte **Profil** z [knižnice](profiles) (I, T, L, vlastný), alebo **Typ** nosníka.
2. Kliknite na začiatok nosníka, potom na jeho koniec. Ďalšími kliknutiami reťazíte nosníky. `Esc` zastaví a výber iného nástroja (alebo znova Nosník) začne nový reťazec.

Predvolene je vrch nosníka na vrchu podlažia (*Odsadenie vrchu* 0), visí pod ďalším podlažím. Jeho úchyty posúvajú každý koniec a vrch.

### Nosníky na ploche {#beams-on-a-face}

Zapnite **Na ploche** vo voľbách nástroja a kreslite v 3D na plochu: strešnú rovinu, stenu. Prvé kliknutie vyberie plochu a nosník leží v jej rovine, kolmo na ňu a na jej povrchu, pod ľubovoľným uhlom, ktorý nakreslíte. Ďalšie body zostávajú na tej ploche, takže väznice, krokvy a vzpery idú tam, kam patria. Nosník nakreslený na streche patrí podlažiu pod ňou. *Zvislo* vo *Vlastnostiach* znova postaví jeho prierez zvislo.

### Reťazce a rohy {#chains-and-corners}

**Rohy** vo voľbách nástroja určujú, ako sa nosníky reťazca stretnú:

| Rohy | Výsledok |
| --- | --- |
| **Samostatne** | Samostatné kusy, ako betónové nosníky |
| **Na pokos** | Konce zrezané na čiare medzi nimi, ako zváraná oceľ |
| **Na tupo** | Jeden prechádza, ďalší končí oň |

### Sklony, predĺženia a konce {#slopes-extensions-and-ends}

*Vlastnosti* majú **Výšku na začiatku** a **Výšku na konci** na sklon nosníka (krokva, schodnica) a **Predĺženie začiatku** a **Predĺženie konca** na predĺženie konca za nakreslený bod, alebo zápornou hodnotou na skrátenie. **Konce** sú *Rovné*, *Jeden na pokos* alebo *Oba na pokos*; *Rovné konce* odstráni pokosy.

## Vo výkresoch a výkazoch {#in-drawings-and-schedules}

Prerezané stĺpy sú v pôdorysoch plne vyplnené; nosníky nad rezom sú čiarkované a prvky viditeľné zhora majú farbu svojho materiálu. Šikmý nosník je prerezaný a kreslený celou výškou. Základy pod stĺpmi pozri v [Základy](footings). *Súbor › Nový výkaz › Stĺpy a nosníky* vypíše každý s typom, podlažím, materiálom, prierezom a dĺžkou.

Nosník v stene je vo väčšine výkresov skrytý: na jeho zobrazenie použite [röntgen](visibility#x-ray) alebo *Skryté* v pohľade a `Tab` na jeho [výber](select#picking-what-is-underneath).
