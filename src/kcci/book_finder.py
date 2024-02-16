from typing import List
import requests

"""
For a given `(title, authors)` pair find the most likely candidate from Open Library.
"""

class Book:
    def __init__(self, title: str, authors: List[str], description: str) -> None:
        self.title = title
        self.authors = authors
        self.description = description

def openlibrary_lookup(name: str, authors: List[str]) -> Book:
    r = requests.get('http://openlibrary.org/search.json', params={
        'title': name,
        'author': authors,
    })
    if r.status_code != 200:
        raise Exception('Bad response from server: %s' % r.status_code)

    data = r.json()
    
    title = data['docs'][0]['title']
    authors = data['docs'][0]['author_name']    

    key = data['docs'][0]['key']

    r = requests.get('http://openlibrary.org%s.json' % key)
    if r.status_code != 200:
        raise Exception('Bad response from server: %s' % r.status_code)

    doc = r.json()
    desc = doc['description']
    if type(desc) == dict:
        desc = doc['description']['value']
    
    return Book(title, authors, desc)


def loc_lookup(name: str, authors: List[str]) -> Book:
    r = requests.get('https://www.loc.gov/books/', params={
        'all': 'true',
        'q': name,
        'fo': 'json',
        'fa': 'contributor:{}'.format(authors[0]),
    })
    if r.status_code != 200:
        raise Exception('Bad response from server: %s' % r.status_code)

    return r.json()


if __name__ == '__main__':
    book = openlibrary_lookup('Spinning Silver', ['Naomi Novik'])
    #print(book.title)
    #print(book.authors)
    #print(book.description)

    b = loc_lookup('Spinning Silver', ['Naomi Novik'])
