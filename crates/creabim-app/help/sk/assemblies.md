# Skladby

Skutočné steny, podlahy a strechy sa skladajú z vrstiev: tehla, vzduchová medzera, tepelná izolácia, tvárnica, omietka. **Skladba** je pomenovaný zoznam týchto vrstiev, každej s materiálom, hrúbkou a funkciou, uložený v projekte a zdieľaný všetkými prvkami, ktoré ho používajú. Pôdorysy a rezy šrafujú každú vrstvu podľa jej materiálu a 3D pohľad zobrazí každú vrstvu ako samostatné teleso.

## Okno Skladby {#the-assemblies-window}

Otvorte ho cez *Materiály › Skladby…* alebo **⋯** pri skupine *Skladby* v prehliadači.

- Skladby sú zoskupené na steny, podlahy a strechy.
- Tabuľka vrstiev ide od vonkajšieho povrchu k vnútornému: funkcia, materiál a hrúbka, s tlačidlami na zmenu poradia a odstránenie vrstiev.
- Náhľad zobrazuje vyšrafované vrstvy so zátvorkou pri **nosnej časti** (nosných vrstvách).
- Skladby môžete vytvárať, duplikovať a odstraňovať, vidieť, koľko prvkov ktorú používa, vybrať ich alebo skladbu priradiť výberu.

![Okno Skladby](img/assemblies.jpg)

*Materiály › Pridať základné skladby* pridá obvodovú tehlovú stenu so vzduchovou medzerou, vnútornú stenu z tvárnic, betónovú stropnú konštrukciu a šikmú strechu so škridlou.

## Použitie {#using-one}

Vyberte steny, dosky alebo strechy a potom vyberte skladbu cez *Materiály › Priradiť skladbu* alebo výberom *Skladba* vo *Vlastnostiach*. Hrúbka sa určí z vrstiev. *Žiadna* vráti jeden materiál.

Vrstvy steny začínajú na jej vonkajšej strane; [otočením](walls#inside-and-outside-flip) strany vymeníte.

## Napojenia {#joins}

Vrstvené steny sa napájajú po vrstvách: rohy L sú na pokos a v styku T nosná časť pokračuje k nosnej časti druhej steny.

## Dvere a okná vo vrstvených stenách {#doors-and-windows-in-layered-walls}

V 3D sú ostenia okolo dverí alebo okna vo vrstvenej stene obložené povrchovými vrstvami steny: vonkajšou na vonkajšej časti ostenia, vnútornou na vnútornej. Tepelná izolácia a fólie sa tak neukazujú tam, kde nimi prechádza otvor. Obklady slúžia len 3D pohľadu; výkresy a exporty zobrazujú vrstvy tak, ako sú.
