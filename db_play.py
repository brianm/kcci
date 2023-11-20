import sqlite3
import sqlite_vss
from sentence_transformers import SentenceTransformer, util
import sys
import numpy as np
import io
import torch


db = sqlite3.connect(':memory:')
db.enable_load_extension(True)
sqlite_vss.load(db)
db.enable_load_extension(False)

titles = ['Dune', 'Ender\'s Game', '48 Laws of Power', 'Dory Fantasmagory']
#Our sentences we like to encode
corpus = [
    """Set on the desert planet Arrakis, Dune is the story of the boy Paul Atreides, heir to a noble 
        family tasked with ruling an inhospitable world where the only thing of value is the "spice" 
        melange, a drug capable of extending life and enhancing consciousness. Coveted across the
        known universe, melange is a prize worth killing for...
        When House Atreides is betrayed, the destruction of Paul's family will set the boy on a journey
        toward a destiny greater than he could ever have imagined. And as he evolves into the mysterious
        man known as Muad'Dib, he will bring to fruition humankind's most ancient and unattainable dream.

        A stunning blend of adventure and mysticism, environmentalism and politics, Dune won the first
        Nebula Award, shared the Hugo Award, and formed the basis of what is undoubtedly the grandest 
        epic in science fiction.""",

    """Ender's Game is a 1985 military science fiction novel by American author Orson Scott Card. Set 
        at an unspecified date in Earth's future, the novel presents an imperiled humankind after two 
        conflicts with the Formics, an insectoid alien species they dub the "buggers". In preparation 
        for an anticipated third invasion, children, including the novel's protagonist, Andrew "Ender" 
        Wiggin, are trained from a very young age by putting them through increasingly difficult games, 
        including some in zero gravity, where Ender's tactical genius is revealed.

        The book originated as a short story of the same name, published in the August 1977 issue of 
        Analog Science Fiction and Fact. The novel was published on January 15, 1985. Later, by 
        elaborating on characters and plotlines depicted in the novel, Card was able to write additional 
        books in the Ender's Game series. Card also released an updated version of Ender's Game in 1991, 
        changing some political facts to reflect the times more accurately (e.g., to include the recent 
        collapse of the Soviet Union and the end of the Cold War). The novel has been translated into 34 
        languages.

        Reception of the book has been mostly positive. It has become suggested reading for many military 
        organizations, including the United States Marine Corps. Ender's Game was recognized as 
        "best novel" by the 1985 Nebula Award[3] and the 1986 Hugo Award[4] in the genres of science 
        fiction and fantasy. Its four sequels—Speaker for the Dead (1986), Xenocide (1991), Children 
        of the Mind (1996), and Ender in Exile (2008)—follow Ender's subsequent travels to many different 
        worlds in the galaxy. In addition, the later novella A War of Gifts (2007) and novel Ender's 
        Shadow (1999), plus other novels in the Shadow saga, take place during the same time period as 
        the original.""",
    
    """Amoral, cunning, ruthless, and instructive, this piercing work distills three thousand years of 
        the history of power in to forty-eight well explicated laws. As attention--grabbing in its design 
        as it is in its content, this bold volume outlines the laws of power in their unvarnished essence, 
        synthesizing the philosophies of Machiavelli, Sun-tzu, Carl von Clausewitz, and other great 
        thinkers. Some laws teach the need for prudence ("Law 1: Never Outshine the Master"), the virtue 
        of stealth ("Law 3: Conceal Your Intentions"), and many demand the total absence of mercy 
        ("Law 15: Crush Your Enemy Totally"), but like it or not, all have applications in real life. 
        Illustrated through the tactics of Queen Elizabeth I, Henry Kissinger, P. T. Barnum, and other 
        famous figures who have wielded--or been victimized by--power, these laws will fascinate any 
        reader interested in gaining, observing, or defending against ultimate control.""",
    
    """Dory, the youngest in her family, is a girl with a very active imagination, and she spends the 
        summer playing with her imaginary friend, pretending to be a dog, battling monsters, and 
        generally driving her family nuts.

        As the youngest in her family, Dory really wants attention, and more than anything she wants 
        her brother and sister to play with her. But she's too much of a baby for them, so she's left 
        to her own devices."""
]

db.execute('''
    create virtual table vss_play using vss0 (
        desc_embedding(768)
    )
    ''')
db.execute('''
    create table play (
        rowid integer primary key,
        title text
    )''')

model = SentenceTransformer('msmarco-distilbert-base-v4')
id = 0
for desc in corpus:
    embedding = model.encode(desc)    
    db.execute('insert into play (rowid, title) values (?, ?)', (id, titles[id]))
    db.execute('insert into vss_play (rowid, desc_embedding) values (?, ?)', (id, embedding))
    db.commit()
    id = id + 1

query = " ".join(sys.argv[1:])

#top_k = min(1, len(corpus))
qe = model.encode(query)
rs = db.execute('''
            with matches as (
                select rowid, distance 
                from vss_play where vss_search(desc_embedding, ?)
                limit 2
            )
            select 
                play.title as title,
                matches.distance as distance
            from play inner join matches using (rowid)             
            order by distance desc
            limit 2
        ''', (qe,))
for title, distance in rs:
    print(f'{title}: {distance}')
