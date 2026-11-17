# Viditeľnosť a röntgen

## Čo pohľad zobrazuje {#what-a-view-shows}

Každý pohľad si uchováva vlastný zoznam toho, čo vynecháva:

- celé **kategórie**: steny, dosky, dvere, okná, stĺpy, nosníky, strechy, schodiská, zábradlia, objemy, rezné dutiny, osi, podlažia, značky rezov, kóty, značky, poznámky, náčrty, pomocné čiary;
- jednotlivé **osové systémy**;
- jednotlivé **prvky**.

Nastavíte ho ponukou *Viditeľnosť* v hlavičke oblasti alebo vo *Vlastnostiach* (karta *Pohľad*), keď nie je nič vybrané. Alebo kliknite pravým tlačidlom na prvok a zvoľte *Skryť v pohľade* alebo *Skryť kategóriu v pohľade*. *Vlastnosti* vypisujú skryté prvky, každý s tlačidlom *Zrušiť skrytie*.

Ak chcete v pohľade ponechať len niekoľko prvkov, vyberte ich, kliknite pravým tlačidlom a zvoľte **V tomto pohľade zobraziť len výber**: pohľad a každý list, na ktorom je, ich zobrazuje samotné, kým znova nezobrazíte ostatné.

Toto sa ukladá s pohľadom a vracia sa ako každá úprava. Listy zobrazujú každý pohľad tak, ako je nastavený, takže čokoľvek pohľad skrýva, je skryté na každom liste, na ktorom je.

## 3D pohľad {#the-3d-view}

3D pohľad má vlastný zoznam: *Zobraziť* v navigačných tlačidlách alebo *Pohľad › Viditeľnosť v 3D pohľade*. Najprv sú rýchle prepínače: *Všetky 2D objekty*, *Osi*, *Anotácia*, *Pomocné čiary*, *Náčrty*, *Rezné dutiny*, *Mriežka terénu*, *2D pohľad v 3D priestore*, *Röntgen*, *Skryté prvky farebne*; potom každá kategória. Vaša voľba sa zapamätá.

Kliknite pravým tlačidlom na prvok v 3D a zvoľte *Skryť v 3D pohľade*, ak ho chcete vynechať len z 3D pohľadu.

## Pomocné čiary zapnúť a vypnúť {#guides-on-and-off}

*Pohľad › Pomocné čiary* (`Shift+G`) skryje všetky pomocné čiary vo všetkých pohľadoch bez ich odstránenia. Opätovným stlačením alebo výberom nástroja Pomocná čiara ich vrátite.

## Röntgen {#x-ray}

Röntgen umožňuje vidieť, vyberať a presúvať to, čo je ukryté vnútri iných prvkov.

- **V 3D**: `Alt+Z`, tlačidlo Röntgen v navigačnej lište alebo *Pohľad › Röntgen*. Steny, dosky, strechy a objemy sa stanú priehľadnými s hranami; nosníky, stĺpy a výber zostávajú plné. Kliknutia uprednostnia nosnú konštrukciu (nosníky, stĺpy, prúty) pred stenami okolo nej.
- **V pôdoryse, reze alebo pohľade**: `Alt+Z` s ukazovateľom nad pohľadom alebo *Röntgen* v lište volieb nástroja. Steny, dosky a strechy zoslabnú na jemné výplne, prvky, ktoré skrývajú, sa kreslia plnými čiarami a sú pod ukazovateľom prvé, takže ich môžete vybrať a potiahnuť.

2D röntgen je spôsob práce: neukladá sa a nikdy sa nezobrazí na listoch.

![Röntgen v reze B-B: steny a dosky priehľadné, nosníky a stĺpy v nich plné](img/xray-2d.jpg)

## Skryté prvky {#hidden-elements}

- **Skryté** (hlavička oblasti alebo *Prvky skryté v iných, čiarkovane* vo viditeľnosti pohľadu) kreslí hrany prvkov skrytých vnútri alebo za inými čiarkovane: nosník v stene, stĺp v stene v pôdoryse, hrana dosky za stenou.
- **Farebne** pridá pod čiarkované čiary každého skrytého prvku priehľadnú výplň vo farbe jeho materiálu, takže drevený preklad v stene je rozpoznateľný ako drevo. Kliknutím do tónu ho vyberiete. Zapnutím sa zapne aj *Skryté*.

Obe sa ukladajú s pohľadom a tlačia sa na listy, do PDF aj SVG. V 3D kreslí *Skryté prvky farebne* (v ponuke *Zobraziť*) ukryté prvky v ich farbách, napoly priehľadne.

## Dočasné skrytie {#hiding-for-a-moment}

`H`, `Shift+H` a `Alt+H` skrývajú a zobrazujú len počas relácie. Pozri [Kopírovanie, vloženie, odstránenie a späť](edit#hide-and-show).
