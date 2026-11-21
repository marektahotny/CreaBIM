# Kopírovanie, vkladanie, odstraňovanie a krok späť

## Späť a znovu {#undo-and-redo}

Každú zmenu možno vrátiť: `Ctrl+Z` vráti, `Ctrl+Y` (alebo `Ctrl+Shift+Z`) zopakuje. Ponuka *Upraviť* povie, čo sa vráti („Späť Posunúť stenu“). Ťahanie, posun alebo zmena hodnoty je jeden krok. História sa ukladá v súbore projektu, takže je k dispozícii aj po opätovnom otvorení.

## Kopírovanie, vystrihnutie a vloženie {#copy-cut-and-paste}

- `Ctrl+C` skopíruje výber, `Ctrl+X` ho vystrihne a `Ctrl+V` ho vloží späť na to isté miesto. Steny si ponechajú dvere a okná.
- Potom stlačte `M` alebo `G` a vloženú kópiu posuňte na miesto.
- Kópia je text, takže ju môžete vložiť aj do druhého okna CreaBIM.

`Shift+D` (duplikovať) skopíruje a hneď začne kópiu posúvať. Pri Posune `Ctrl` zanechá kópiu pri každom kliknutí.

## Rozdeliť {#split}

**Rozdeliť** rozreže prvok na dva pozdĺž čiary, ktorú zvolíte, kým [Rozčleniť](divide) ho reže na rovnaké časti.

1. Vyberte, čo chcete rozrezať: steny, dosky, diely, náčrty alebo hmoty.
2. Pravým tlačidlom naň kliknite a zvoľte **Rozdeliť…** (hore v ponuke), použite *Upraviť › Rozdeliť…* alebo tlačidlo Rozdeliť vedľa Duplikovať.
3. Kliknite na pomocnú čiaru, čiaru alebo hranu, pozdĺž ktorej rezať, alebo kliknite na dva body. Pri ich výbere funguje prichytávanie. Červená priehľadná rezná rovina ukazuje, kde reže, v každom pohľade.
4. Kliknite na stranu, ktorá má zostať vybraná.

Steny sa rozrežú tam, kde ich čiara pretína rovinu. Dosky, diely nakreslené z pôdorysu a uzavreté náčrty sa rozrežú na obrysy na každej strane (rez cez obe ramená tvaru U dá tri). Otvorený náčrt sa rozreže v každom priesečníku a hmota rovinou. Rozdelenie je jeden krok späť; `Esc` ho opustí.

## Odstránenie {#delete}

`Del` odstráni vybrané. Odstráni sa aj to, čo k tomu patrí: dvere a okná steny, značky a pripojené kóty prvku. Tlačidlo *Odstrániť* vo *Vlastnostiach* ich spočíta („Odstrániť (a 2 pripojené)“). `Ctrl+Z` vráti všetko.

## Skrytie a zobrazenie {#hide-and-show}

Ak chcete niečo na chvíľu odložiť z cesty:

- `H` skryje výber, v 3D aj v 2D pohľadoch.
- `Shift+H` zobrazí iba výber.
- `Alt+H` znova zobrazí všetko.

Slúži to iba na prácu: listy a exporty stále zobrazujú všetko a skryté prvky sa vrátia po opätovnom otvorení projektu. Ak chcete veci z pohľadu vynechať natrvalo, použite [viditeľnosť](visibility).
