# Závesné steny

**Závesná stena** je stena z mriežky stĺpikov a priečnikov, ktoré nesú sklo alebo plné panely: výklad, presklená fasáda.

## Kreslenie {#drawing-one}

Použite *Vytvoriť › Závesná stena*: spustí nástroj Stena s typom závesnej steny. Alebo vyberte ktorýkoľvek typ závesnej steny v ponuke **Typ** nástroja Stena. Potom ju nakreslite ako ktorúkoľvek stenu.

![Závesná stena v 3D s nastaveniami vo Vlastnostiach](img/curtain-wall.jpg)

## Jej mriežka {#its-grid}

Vyberte ju. V časti *Závesná stena* vo *Vlastnostiach*:

- **Pozdĺž**: polia pozdĺž steny, počtom alebo rozostupom;
- **Nahor**: rady po výške steny, počtom alebo rozostupom;
- šírka a hĺbka stĺpikov;
- sklenené alebo plné panely a hrúbka panelu.

Tieto hodnoty zvyčajne pochádzajú z jej [typu](types), takže zmena typu zmení každú závesnú stenu tohto typu.

## Osadenie v stene {#set-in-a-wall}

Nakreslite závesnú stenu pozdĺž steny alebo ju do nej presuňte a vyreže si v nej vlastný otvor ako dvere či okno: stena príde o dĺžku a výšku závesnej steny. Otvor sleduje závesnú stenu pri presune a zavrie sa, keď stenu opustí. *Vlastnosti* uvádzajú, v ktorej stene je osadená. V IFC dostane stena `IfcOpeningElement`, ktorý závesná stena vypĺňa.

Závesná stena nakreslená samostatne stojí samostatne, ako doteraz.

## Dvere v nej {#doors-in-it}

Dvere alebo okno umiestnite do závesnej steny ako do ktorejkoľvek steny: osadia sa do panelu.

Závesné steny sa kreslia v pôdorysoch, rezoch, pohľadoch aj v 3D a do IFC sa exportujú ako `IfcCurtainWall` so stĺpikmi a panelmi.
