# Úchyty

Vybraný prvok zobrazuje malé úchyty, ťahaním ktorých ho priamo meníte, v 3D, pôdorysoch aj rezoch.

- **Štvorcové úchyty** posúvajú bod: konce steny, rohy a stredy hrán dosky či strechy, konce nosníka, konce osi, pätu stĺpa, začiatok schodiska, body zábradlia.
- **Okrúhle úchyty** menia rozmer: výšku a líca steny, ostenia (šírku), nadpražie a parapet dverí či okna, vrch stĺpa.

Úchyty sa prichytávajú k tomu, na čo dopadnú, a údaj ukazuje novú hodnotu. Každé ťahanie je jeden krok späť.

Ťahanie je voľné: prvok ide presne tam, kam ho ukazovateľ dovedie, a prichytenie ho zachytí, iba keď sa ukazovateľ priblíži k rohu, hrane, pomocnej čiare alebo osi. Podržaním `Alt` ťaháte úplne bez prichytávania. Keď chcete okrúhle čísla, sú k dispozícii dve pomôcky, obe vypnuté, kým ich nezapnete v ponuke **Prichytenie k mriežke** v stavovom riadku:

- **Prichytenie k mriežke** (`Alt+S`): body a ťahania padnú na mriežku zvoleného rozostupu (10 mm, 50 mm, 100 mm alebo vlastný), meranú od počiatku projektu.
- **Značky vzdialenosti**: počas ťahania slabé značky v každom rozostupe od miesta uchopenia; pri jednej z nich sa ťahanie zastaví.

Presnú hodnotu zadáte počas ťahania (`Tab`) alebo kliknutím na dočasnú kótu.

## Váš štýl úchytov {#your-grip-style}

*CreaBIM › Nastavenia… › Úchopy* mení vzhľad úchytov, iba pre vás: *Tichý* (štvorce v rohoch, pruhy na hranách), *Hrany sú úchyty* (v pokoji takmer nič; hrana ukáže svoj pruh pod ukazovateľom) alebo *Jeden tvar úchytu* (všade rovnaký malý krúžok). Podržaním ukazovateľa nad voľbou ju uvidíte na výbere. Nikdy nemení niečie výkresy a ide s vami z projektu do projektu.

## Otáčanie, posúvanie a zmena rozmerov {#turning-sliding-and-sizing}

- Vybraný **objekt** (nábytok alebo zariaďovací predmet) má pred sebou štvorcový úchyt: ťahaním dookola ho otáčate okolo jeho stredu, po 5°. Šípka vedľa neho ho otočí opačným smerom.
- **Stĺp** má vedľa seba úchyt otáčania a okrúhle úchyty na bokoch pre šírku a hĺbku (pri kruhovom stĺpe priemer); zostáva na svojom bode.
- **Schodisko** má úchyt otáčania za prvým stupňom.
- **Dvere alebo okno** majú v strede okrúhly úchyt na posun pozdĺž steny pri zachovaní šírky; **nosník** má na boku okrúhly úchyt pre šírku.
- **Stena** má okrúhly úchyt pri líci oproti úchytu hrúbky: ťahaním posuniete celú stenu kolmo na ňu samu a steny na ňu napojené sa natiahnu, aby zostali napojené. Pravým tlačidlom na stenu a *Rozdeliť stenu tu* vytvoríte dve steny stretávajúce sa v tom bode; dvere a okná zostanú na mieste.

## Steny zostávajú napojené {#walls-stay-joined}

Potiahnite koniec steny a steny na ňu napojené ho nasledujú, takže roh zostane uzavretý. Ak chcete ťahať koniec samostatne, podržte počas ťahania `Ctrl` alebo `Alt`, alebo zakážte napojenie na tom konci (pozri [Steny](walls#joins)).

## Rohy obrysov {#outline-corners}

Dosky, strechy, šachty, náčrty a trasy zábradlí zobrazujú na obryse rovnaké úchyty: štvorec na každom rohu na jeho posun a pri uzavretom obryse okrúhly úchyt na každej hrane na jej posun. Rohy pridáte alebo odstránite, prípadne zmeníte tvar otvorov, v [Režime úprav](edit-mode) (`Enter` alebo dvojklik): vyberte rohy a stlačte `Delete`, potiahnite ◇ a pridáte roh, alebo pravým tlačidlom pre ďalšie možnosti.

Podržte ukazovateľ na ľubovoľnom úchyte a tip povie, čo robí. Počas ťahania `Tab` umožní zadať dĺžku.

## Paleta hrany {#the-edge-palette}

Podržte ukazovateľ na hrane obrysu vybraného prvku, v 3D pohľade alebo pôdoryse, a pri ukazovateli sa otvorí malá paleta:

- **Odsadiť hranu** vysunie alebo vtiahne hranu kolmo na ňu samu.
- **Odsadiť všetko** zväčší alebo zmenší celý obrys o jednu vzdialenosť, na mieste. `Ctrl` namiesto toho vytvorí kópiu.
- **Vložiť bod** pridá roh na tomto mieste a potom ho posúva.

Každá voľba sleduje ukazovateľ s prichytením. Zadajte vzdialenosť a `Enter` ju pošle tak ďaleko smerom ukazovateľa, kliknutím umiestnite, alebo `Esc` zastaví.

## Šachty {#shafts}

Vybraná šachta má štvorcový úchyt na každom rohu, okrúhly na každej hrane a okrúhle úchyty pre spodok a vrch, v každom pôdoryse, reze a 3D pohľade, ktorým prechádza.

## V rezoch {#in-sections}

Vyberte prvok v reze: jeho pôdorysné úchyty sa posúvajú pozdĺž čiary rezu a výškové úchyty hore a dole.

## Čiary rezov a pohľadov {#section-and-elevation-lines}

Vyberte značku rezu alebo pohľadu v pôdoryse: štvorcové úchyty posúvajú jej konce, okrúhly stredný úchyt posúva celú čiaru a úchyt na tieňovanom páse nastavuje hĺbku pohľadu. Pozri [Rezy a pohľady](sections).

## Orezové kvádre {#crop-boxes}

Vybraný orezový kváder má okrúhle úchyty na stenách a štvorcový úchyt v rohu na posun. Pozri [Orezové kvádre](crop-boxes).
