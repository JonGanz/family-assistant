"""Tiny UML 2 draw.io generator. Each arch/src/<Name>.py builds pages and writes arch/<Name>.drawio.
Usage: python3 arch/src/<Name>.py  (run from repo root)"""
from xml.sax.saxutils import escape as _e
import xml.dom.minidom

NODE = "shape=cube;whiteSpace=wrap;html=1;boundedLbl=1;size=12;verticalAlign=top;align=left;spacingLeft=8;fontStyle=1;"
COMP = "shape=component;align=left;spacingLeft=36;html=1;whiteSpace=wrap;"
EXT = COMP + "dashed=1;fillColor=#f5f5f5;"
ART = "shape=note;size=12;html=1;whiteSpace=wrap;"
FRAME = "shape=umlFrame;whiteSpace=wrap;html=1;width=260;height=24;verticalAlign=top;align=left;spacingLeft=8;"
TITLE = "text;fontSize=16;fontStyle=1;html=1;"
NOTE = "shape=note;size=14;html=1;whiteSpace=wrap;align=left;spacingLeft=6;fillColor=#fff2cc;"


class Page:
    _seq = [1]

    def __init__(self, pid, name, w=1400, h=900):
        self.pid, self.name, self.w, self.h, self.cells = pid, name, w, h, []

    def _id(self):
        Page._seq[0] += 1
        return f"c{Page._seq[0]}"

    def box(self, label, x, y, w, h, style):
        i = self._id()
        self.cells.append(f'<mxCell id="{i}" value="{_e(label)}" style="{style}" vertex="1" parent="1">'
                          f'<mxGeometry x="{x}" y="{y}" width="{w}" height="{h}" as="geometry"/></mxCell>')
        return i

    def edge(self, s, t, label="", dashed=True):
        st = "endArrow=open;endSize=12;html=1;" + ("dashed=1;" if dashed else "")
        self.cells.append(f'<mxCell id="{self._id()}" value="{_e(label)}" style="{st}" edge="1" parent="1" source="{s}" target="{t}">'
                          '<mxGeometry relative="1" as="geometry"/></mxCell>')

    def xml(self):
        return (f'<diagram id="{self.pid}" name="{self.name}"><mxGraphModel dx="{self.w}" dy="{self.h}" grid="1" gridSize="10" page="1" '
                f'pageWidth="{self.w}" pageHeight="{self.h}"><root><mxCell id="0"/><mxCell id="1" parent="0"/>{"".join(self.cells)}</root></mxGraphModel></diagram>')


def write(path, pages):
    out = '<mxfile host="drawio" version="24.0.0">' + "".join(p.xml() for p in pages) + "</mxfile>"
    xml.dom.minidom.parseString(out)  # validate
    open(path, "w").write(out)
    print(f"wrote {path} ({len(pages)} pages)")
