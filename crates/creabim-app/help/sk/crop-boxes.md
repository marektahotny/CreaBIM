# Orezové kvádre

**Orezový kváder** oreže 3D pohľad na to, čo je v ňom, aby ste videli dovnútra budovy: jedno podlažie, jednu miestnosť, detail. Je ako orezový kváder (section box) v Reville, ale s viacerými možnosťami:

- môže orezať **všetko** alebo **len vybrané prvky**;
- pohľad môže mať **ľubovoľný počet kvádrov**; prvok si ponechá len to, čo je vnútri všetkých kvádrov, ktoré ho orezávajú;
- každý kváder možno **vypnúť** (zostane, ale nič neoreže) alebo **skryť** (jeho rám zmizne, ale stále orezáva).

Prerezané plochy sa uzatvárajú tmavším uzáverom, takže steny a dosky pôsobia plné, vrstva po vrstve.

## Pridanie {#adding-one}

- *Pohľad › Orezové kvádre › Nový orezový kváder*: kváder okolo celého modelu.
- *Nový orezový kváder okolo výberu*: okolo vybraných prvkov, orezáva všetko.
- *Nový orezový kváder len pre výber*: orezáva len vybrané prvky.
- Alebo *Pridať orezový kváder* vo *Vlastnostiach*, keď nie je nič vybrané, alebo **+** pri skupine *Orezové kvádre* v prehliadači.

## Úprava tvaru {#shaping-it}

- Potiahnite okrúhle úchopy na jeho stenách; prichytávajú sa k modelu.
- Potiahnutím štvorcového úchopu v rohu ho presuniete po pôdoryse, alebo stlačte `G`.
- Alebo zadajte jeho rozsah vo *Vlastnostiach*: *X*, *Y* a *Z od … do …* v jednotkách projektu. *Prispôsobiť modelu* ho znova prispôsobí.

![Nový orezový kváder okolo základného domu: v 3D, čiarkovane v pôdoryse prízemia a jeho nastavenia vo Vlastnostiach](img/crop-box.jpg)

## Orezanie len niektorých prvkov {#cropping-only-some-elements}

Nastavte *Čo oreže › Len vybrané prvky*, kliknite na **Vybrať prvky…** a potom klikaním na prvky v 3D pohľade ich pridávajte (opätovným kliknutím ponecháte prvok celý). Výber steny zahŕňa jej dvere a okná. `Esc`, pravé tlačidlo alebo **Výber hotový** ukončí výber.

## Správa kvádrov {#managing-boxes}

Keď nie je nič vybrané, *Vlastnosti* vypisujú kvádre, každý s voľbami *Orezať*, *Zobraziť kváder*, názvom a *Odstrániť*. *Pohľad › Orezové kvádre* ich vypisuje so zaškrtávaním zapnutia.

## V pôdorysoch a rezoch {#in-plans-and-sections}

Orezový kváder sa zobrazuje v pôdorysoch (jeho pôdorys) a v rezoch a pohľadoch (jeho rozsah v nich), čiarkovane vo farbe orezania: vybraný a každý kváder so zapnutým *Zobraziť kváder*. Kliknutím na obrys ho vyberiete a jeho úchopy stien tam môžete ťahať ako v 3D; 3D pohľad ho nasleduje.

## Čo sa zobrazuje mimo {#what-shows-outside}

**Vonku** pre každý kváder určuje, čo sa zobrazí z prvkov, ktoré oreže celé: **Skryté**, **Priesvitne pri výbere** (predvolené: vybraný prvok mimo sa zobrazí ako čiarkovaný kváder s jemnou výplňou) alebo **Vždy priesvitne**.

Keď vybraný prvok leží mimo aktívneho kvádra, hore v 3D pohľade sa to oznámi s tlačidlami **Zväčšiť kváder** (jeden krok pre Späť), **Aj tak zobraziť** (kváder odvtedy ponechá tento prvok celý) a **Vypnúť orezanie**.

## Čie kvádre {#whose-boxes}

Keď je v oblasti otvorený uložený 3D pohľad, 3D pohľad zobrazuje a upravuje orezové kvádre tohto pohľadu a kresba pohľadu (a každý list, na ktorom je) je orezaná rovnako. Inak 3D pohľad používa vlastné kvádre.

Orezové kvádre ovplyvňujú len 3D pohľady. Exporty (IFC, glTF, FreeCAD) sa nikdy neorezávajú.
