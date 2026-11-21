# Podlažia

**Podlažie** je poschodie budovy v určitej výške, napríklad *Prízemie* na ±0 alebo *1. poschodie* na +3000. Každý prvok stojí na podlaží s odsadením od neho. Posuňte podlažie a posunie sa všetko na ňom. Každé podlažie má zvyčajne vlastný pôdorys.

## Okno Podlažia {#the-levels-window}

Otvoríte ho cez *Pohľad › Podlažia…* (aj v *Súbor* a *Vytvoriť*), **⋯** pri skupine *Podlažia* v prehliadači, *Podlažia…* vo *Vlastnostiach* podlažia alebo výberom nástroja Podlažie kdekoľvek okrem rezu.

- Podlažia sú zoradené od najvyššieho, s názvom, výškovou úrovňou, konštrukčnou výškou, odstupom k ďalšiemu podlažiu (s **Prispôsobiť**, keď sa líšia) a počtom prvkov na každom.
- **Pridať podlažie nad najvyšším** pridá podlažie o poschodie vyššie. **Pridať na** pridá podlažie na zadanú výškovú úroveň. So zaškrtnutým *Vytvoriť pôdorys pre každé nové podlažie* dostane každé aj svoj pôdorys.
- Podlažie premenujte, zmeňte jeho výškovú úroveň alebo výšku, duplikujte ho, otvorte alebo vytvorte jeho pôdorys.
- **Odstrániť** sa spýta, čo sa stane s tým, čo je na ňom: presunúť tieto prvky na iné podlažie (zostanú v priestore tam, kde sú) alebo ich tiež odstrániť.

![Okno Podlažia](img/levels.jpg)

Pôdorysy pomenované podľa podlažia ho pri premenovaní nasledujú („Pôdorys Podlažie 2“ sa zmení na „Pôdorys Strecha“).

## Podlažia v rezoch {#levels-in-sections}

V reze alebo pohľade je každé podlažie čiara so záhlavím s jeho názvom a výškovou úrovňou.

- Ťahaním čiary podlažia (alebo jej záhlavia) hore či dole posuniete podlažie a všetko na ňom. Prichytáva sa k bodom modelu.
- Nástrojom **Podlažie** kliknite tam, kde začína čiara nového podlažia (tým nastavíte jeho výšku), potom tam, kde končí. Spolu s ním sa vytvorí jeho pôdorys.

## Steny viazané na podlažia {#walls-tied-to-levels}

Vrch steny možno nastaviť *Po* podlažie: potom stena rastie alebo sa zmenšuje, keď sa toto podlažie posunie. Pozri [Steny](walls#base-and-top).

## Kopírovanie a presun na iné podlažia {#copying-and-moving-to-other-levels}

Nakreslite niečo raz na jednom podlaží a potom to dajte na ďalšie bez opätovného kreslenia.

1. Vyberte, čo chcete skopírovať: steny, dosky, stĺpy, nosníky, schodiská, základy, nábytok a zariaďovacie predmety, alebo celé podlažie (`Ctrl+A` v jeho pôdoryse).
2. Pravým tlačidlom zvoľte **Kopírovať na podlažia…**. Je aj v ponuke *Upraviť*.
3. Zaškrtnite požadované podlažia, alebo použite **Všetko nad** či **Všetko pod**, potom kliknite na **Kopírovať na** *n* **podlaží**.

Každá kópia sedí na svojom novom podlaží tak ako originál na svojom: rovnaké miesto v pôdoryse, rovnaké odsadenie spodku. Vrch viazaný na podlažie sa posunie hore alebo dole o rovnaký počet podlaží. Napríklad stena na Podlaží 5 vedená *Po* Podlažie 6, skopírovaná na Podlažie 3, pôjde po Podlažie 4. Ak tak vysoko podlažie nie je, vrch zostane nepripojený, ale ponechá si výšku. Dvere a okná idú so svojimi stenami a základy so svojimi stenami a stĺpmi. Všetky kópie sú jeden krok: `Ctrl+Z` ich vráti spolu.

**Presunúť na podlažie…** robí to isté, ale výber presunie namiesto kopírovania. Zvoľte podlažie a prvky sa naň presunú aj s dverami, oknami a základmi.

**Upraviť › Vložiť zarovnané k podlažiu** vloží to, čo ste skopírovali cez `Ctrl+C`, na zvolené podlažie, na rovnaké miesto v pôdoryse. Použite to na prenesenie niečoho z iného podlažia alebo iného súboru.

## Stĺpy cez viac podlaží {#columns-over-several-levels}

Stĺp má **spodné podlažie** a **horné podlažie**, takže môže prechádzať viacerými podlažiami. Pozri [Stĺpy](columns-beams#over-several-levels).

## Podklad {#underlay}

Pôdorys môže slabo zobrazovať iné podlažie pod sebou, na obkresľovanie a prichytávanie. Zvoľte ho v lište volieb nástroja (*Podklad*: Žiadne, Podlažie nižšie, Podlažie vyššie alebo ľubovoľné podlažie), vo *Vlastnostiach* pohľadu alebo v *Pohľad › Podklad*.
