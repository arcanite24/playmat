import requests
import re
import os
from time import sleep
import argparse

parser = argparse.ArgumentParser(
                    prog = 'CubeCobraDownloader',
                    description = 'Given an url for a cube on CubeCobra.com, downloads the cards and helps you implement and push it to the xmage github',
                    epilog = 'Author: TheBear132')

parser.add_argument('url', help="The URL for cube on CubeCobra (must be the overview tab) e.g. https://cubecobra.com/cube/overview/df")

args = parser.parse_args()


MageFolder = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
cards_data_path = os.path.join(MageFolder, "Utils", "mtg-cards-data.txt")
if not os.path.isfile(cards_data_path):
    raise SystemExit(f"Could not find {cards_data_path}; run this script from the repository's Utils directory.")


def clean(varStr): return re.sub(r'\W|^(?=\d)', '', varStr)


def getDecklist(url):
    r = requests.get(url).content.decode()
    #with open("idk.html", "w", encoding="utf8") as f:
    #    f.write(r)
    cubeId, cubeName, owner = re.findall("""{"cube":{"_id":"([0-9a-z]*)",".*"name":"(.*)","owner".*"owner_name":"(.*)","date_updated""", r)[0]

    cubeCards = f"https://cubecobra.com/cube/download/plaintext/{cubeId}?primary=Color%20Category&secondary=Types-Multicolor&tertiary=Mana%20Value&quaternary=Alphabetical&showother=false"
    deck = requests.get(cubeCards)
    deck = deck.content.decode().split("\r\n")
    while("" in deck):
        deck.remove("")
    return (deck, cubeName, owner)


CobraOverview = "https://cubecobra.com/cube/overview/df"
CobraOverview = args.url
print(f"{CobraOverview=}")

deck, cubeName, owner = getDecklist(CobraOverview)
print(f"\n{owner=} | {cubeName=}")

with open(cards_data_path, encoding="utf-8") as f:
    mtgcarddata = f.read()


cleanOwner = clean(owner.replace(" ", ""))
cleanCubeName = clean(cubeName.replace(" ", ""))
    
JavaFile = f"""package mage.tournament.cubes;

import mage.game.draft.DraftCube;

/**
 * @author {os.path.basename(__file__)} by TheBear132
 */
public class {cleanOwner}s{cleanCubeName} extends DraftCube {{

    public {cleanOwner}s{cleanCubeName}() {{
        super("{owner}'s {cubeName}");      // {CobraOverview}
        
"""

for cardName in deck:
    m = re.search(rf"(?:{re.escape(cardName)}\|)(Khans of Tarkir|Fate Reforged|Dragons of Tarkir)\|", mtgcarddata)
    cardName = cardName.replace("\"", "\\\"")           #Henrie the toolbox... wizards come on
    prefSet = ""
    #print(f"{cardName:30}", end="")
    if m is not None:   #If from a KTK block | Yea i just like that block for some reason
        if m[1] == "Khans of Tarkir": prefSet = "KTK"
        if m[1] == "Fate Reforged": prefSet = "FRF"
        if m[1] == "Dragons of Tarkir": prefSet = "DTK"
    JavaFile += f"""        cubeCards.add(new DraftCube.CardIdentity("{cardName}", "{prefSet}"));
"""

JavaFile += """    }
}"""


idk = os.path.join(
    MageFolder,
    "Mage.Server.Plugins",
    "Mage.Tournament.BoosterDraft",
    "src",
    "mage",
    "tournament",
    "cubes",
    f"{clean(owner)}s{clean(cubeName)}.java",
)
if os.path.exists(idk):
    print("Cube already exists:", idk)
    exit(0)
with open(idk, "w", encoding="utf-8") as f:
    f.write(JavaFile)


print("\n---------------- ADD THE FOLLOWING TO EACH FILE RESPECTIVELY ----------------")
print("[+] Mage.Server/src/test/data/config_error.xml")
print(f"    --> <draftCube name=\"{owner}'s {cubeName}\" jar=\"mage-tournament-boosterdraft.jar\" className=\"mage.tournament.cubes.{cleanOwner}s{cleanCubeName}\"/>")

print("\n[+] Mage.Server/release/config/config.xml")
print(f"    --> <draftCube name=\"{owner}'s {cubeName}\" jar=\"mage-tournament-boosterdraft-${{project.version}}.jar\" className=\"mage.tournament.cubes.{cleanOwner}s{cleanCubeName}\"/>")

print("\n[+] Mage.Server/config/config.xml")
print(f"    --> <draftCube name=\"{owner}'s {cubeName}\" jar=\"mage-tournament-boosterdraft.jar\" className=\"mage.tournament.cubes.{cleanOwner}s{cleanCubeName}\"/>")
print("-----------------------------------------------------------------------------")

print("Cube written to ->", idk)

print("\nValidate changes with Git -> create branch -> create pull request -> play your cube!")



