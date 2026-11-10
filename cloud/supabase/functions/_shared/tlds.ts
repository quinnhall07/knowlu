// The enforcement's TLD list for the free-text policy (`output_policy.ts`; prompt-injection suite
// plan, section 4.3 and its amendments). Data only: nothing here, and nothing that imports it, fetches.
//
// Source: IANA's root-zone list, `https://data.iana.org/TLD/tlds-alpha-by-domain.txt`,
// fetched once by hand on 2026-11-09 and committed as data. Its version line is `IANA_VERSION`, verbatim.
//
// Refresh (a dated edit, on the cadence HANDOFF names): fetch the file again by hand; set
// `IANA_VERSION` to its first line and `IANA_TLDS` to every other line, lowercased, in the file's
// order; give every `xn--` name a row in `IDN_TLDS` with its Unicode form (`node:url`'s
// `domainToUnicode`), written as escapes; then run `output_policy_test.ts`, which checks all three.
//
// The Unicode forms are escapes on purpose: several are right-to-left scripts, which would reorder
// the source on screen if written raw.

export const IANA_SOURCE = "https://data.iana.org/TLD/tlds-alpha-by-domain.txt";

/** The fetched file's first line, verbatim. */
export const IANA_VERSION = "# Version 2026110900, Last Updated Mon Nov  9 07:07:01 2026 UTC";

/** The four reserved names (RFC 2606, RFC 6761) the policy adds; none is in the root zone. */
export const RESERVED_TLDS: readonly string[] = ["example", "test", "invalid", "localhost"];

/** Every name in the fetched file, lowercased, in the file's order (1436 names). */
export const IANA_TLDS: readonly string[] = `
aaa aarp abb abbott abbvie abc able abogado abudhabi ac academy accenture accountant accountants aco
actor ad ads adult ae aeg aero aetna af afl africa ag agakhan agency ai aig airbus airforce airtel
akdn al alibaba alipay allfinanz allstate ally alsace alstom am amazon americanexpress
americanfamily amex amfam amica amsterdam analytics android anquan anz ao aol apartments app apple
aq aquarelle ar arab aramco archi army arpa art arte as asda asia associates at athleta attorney au
auction audi audible audio auspost author auto autos aw aws ax axa az azure ba baby baidu banamex
band bank bar barcelona barclaycard barclays barefoot bargains baseball basketball bauhaus bayern bb
bbc bbt bbva bcg bcn bd be beats beauty beer berlin best bestbuy bet bf bg bh bharti bi bible bid
bike bing bingo bio biz bj black blackfriday blockbuster blog bloomberg blue bm bms bmw bn
bnpparibas bo boats boehringer bofa bom bond boo book booking bosch bostik boston bot boutique box
br bradesco bridgestone broadway broker brother brussels bs bt build builders business buy buzz bv
bw by bz bzh ca cab cafe cal call calvinklein cam camera camp canon capetown capital capitalone car
caravan cards care career careers cars casa case cash casino cat catering catholic cba cbn cbre cc
cd center ceo cern cf cfa cfd cg ch chanel channel charity chase chat cheap chintai christmas chrome
church ci cipriani circle cisco citadel citi citic city ck cl claims cleaning click clinic clinique
clothing cloud club clubmed cm cn co coach codes coffee college cologne com commbank community
company compare computer comsec condos construction consulting contact contractors cooking cool coop
corsica country coupon coupons courses cpa cr credit creditcard creditunion cricket crown crs cruise
cruises cu cuisinella cv cw cx cy cymru cyou cz dad dance data date dating datsun day dclk dds de
deal dealer deals degree delivery dell deloitte delta democrat dental dentist desi design dev dhl
diamonds diet digital direct directory discount discover dish diy dj dk dm dnp do docs doctor dog
domains dot download drive dtv dubai dupont durban dvag dvr dz earth eat ec eco edeka edu education
ee eg email emerck energy engineer engineering enterprises epson equipment er ericsson erni es esq
estate et eu eurovision eus events exchange expert exposed express extraspace fage fail fairwinds
faith family fan fans farm farmers fashion fast fedex feedback ferrari ferrero fi fidelity fido film
final finance financial fire firestone firmdale fish fishing fit fitness fj fk flickr flights flir
florist flowers fly fm fo foo food football ford forex forsale forum foundation fox fr free
fresenius frl frogans frontier ftr fujitsu fun fund furniture futbol fyi ga gal gallery gallo gallup
game games gap garden gay gb gbiz gd gdn ge gea gent genting george gf gg ggee gh gi gift gifts
gives giving gl glass gle global globo gm gmail gmbh gmo gmx gn godaddy gold goldpoint golf goodyear
goog google gop got gov gp gq gr grainger graphics gratis green gripe grocery group gs gt gu gucci
guge guide guitars guru gw gy hair hamburg hangout haus hbo hdfc hdfcbank health healthcare help
here hermes hiphop hisamitsu hitachi hiv hk hkt hm hn hockey holdings holiday homedepot homegoods
homes homesense honda horse hospital host hosting hot hotels hotmail house how hr hsbc ht hu hughes
hyatt hyundai ibm icbc ice icu id ie ieee ifm ikano il im imamat imdb immo immobilien in inc
industries infiniti info ing ink institute insurance insure int international intuit investments io
ipiranga iq ir irish is ismaili ist istanbul it itau itv jaguar java jcb je jeep jetzt jewelry jio
jll jm jmp jnj jo jobs joburg jot joy jp jpmorgan jprs juegos kaufen kddi ke kerryhotels
kerryproperties kfh kg kh ki kia kids kim kindle kitchen kiwi km kn koeln komatsu kosher kp kpmg kpn
kr krd kred kuokgroup kw ky kyoto kz la lacaixa lamborghini lamer land landrover lanxess lasalle lat
latino latrobe law lawyer lb lc lds lease leclerc lefrak legal lego lexus lgbt li lidl life
lifeinsurance lifestyle lighting like lilly limited limo lincoln link live living lk llc llp loan
loans locker locus lol london lotte lotto love lpl lplfinancial lr ls lt ltd ltda lu lundbeck luxe
luxury lv ly ma madrid maif maison makeup man management mango map market marketing markets marriott
marshalls mattel mba mc mckinsey md me med media meet melbourne meme memorial men menu merck
merckmsd mg mh miami microsoft mil mini mint mit mitsubishi mk ml mlb mls mm mma mn mo mobi mobile
moda moe moi mom monash money monster mormon mortgage moscow moto motorcycles mov movie mp mq mr ms
msd mt mtn mtr mu museum music mv mw mx my mz na nab nagoya name navy nba nc ne nec net netbank
netflix network neustar new news next nextdirect nexus nf nfl ng ngo nhk ni nico nike nikon ninja
nissan nissay nl no nokia norton now nowruz nowtv np nr nra nrw ntt nu nyc nz obi observer office
okinawa olayan olayangroup ollo om omega one ong onl online ooo open oracle orange org organic
origins osaka otsuka ott ovh pa page panasonic paris pars partners parts party pay pccw pe pet pf
pfizer pg ph pharmacy phd philips phone photo photography photos physio pics pictet pictures pid pin
ping pink pioneer pizza pk pl place play playstation plumbing plus pm pn pnc pohl poker politie porn
post pr praxi press prime pro prod productions prof progressive promo properties property protection
pru prudential ps pt pub pw pwc py qa qpon quebec quest racing radio re read realestate realtor
realty recipes red redumbrella rehab reise reisen reit reliance ren rent rentals repair report
republican rest restaurant review reviews rexroth rich richardli ricoh ril rio rip ro rocks rodeo
rogers room rs rsvp ru rugby ruhr run rw rwe ryukyu sa saarland safe safety sakura sale salon
samsclub samsung sandvik sandvikcoromant sanofi sap sarl sas save saxo sb sbi sbs sc scb schaeffler
schmidt scholarships school schule schwarz science scot sd se search seat secure security seek
select sener services seven sew sex sexy sfr sg sh shangrila sharp shell shia shiksha shoes shop
shopping shouji show si silk sina singles site sj sk ski skin sky skype sl sling sm smart smile sn
sncf so soccer social softbank software sohu solar solutions song sony soy spa space sport spot sr
srl ss st stada staples star statebank statefarm stc stcgroup stockholm storage store stream studio
study style su sucks supplies supply support surf surgery suzuki sv swatch swiss sx sy sydney
systems sz tab taipei talk taobao target tatamotors tatar tattoo tax taxi tc tci td tdk team tech
technology tel temasek tennis teva tf tg th thd theater theatre tiaa tickets tienda tips tires tirol
tj tjmaxx tjx tk tkmaxx tl tm tmall tn to today tokyo tools top toray toshiba total tours town
toyota toys tr trade trading training travel travelers travelersinsurance trust trv tt tube tui
tunes tushu tv tvs tw tz ua ubank ubs ug uk unicom university uno uol ups us uy uz va vacations vana
vanguard vc ve vegas ventures verisign versicherung vet vg vi viajes video vig viking villas vin vip
virgin visa vision viva vivo vlaanderen vn vodka volvo vote voting voto voyage vu wales walmart
walter wang wanggou watch watches weather weatherchannel web webcam weber website wed wedding weibo
weir wf whoswho wien wiki williamhill win windows wine winners wme woodside work works world wow ws
wtc wtf xbox xerox xihuan xin xn--11b4c3d xn--1ck2e1b xn--1qqw23a xn--2scrj9c xn--30rr7y xn--3bst00m
xn--3ds443g xn--3e0b707e xn--3hcrj9c xn--3pxu8k xn--42c2d9a xn--45br5cyl xn--45brj9c xn--45q11c
xn--4dbrk0ce xn--4gbrim xn--54b7fta0cc xn--55qw42g xn--55qx5d xn--5su34j936bgsg xn--5tzm5g
xn--6frz82g xn--6qq986b3xl xn--80adxhks xn--80ao21a xn--80aqecdr1a xn--80asehdb xn--80aswg
xn--8y0a063a xn--90a3ac xn--90ae xn--90ais xn--9dbq2a xn--9et52u xn--9krt00a xn--b4w605ferd
xn--bck1b9a5dre4c xn--c1avg xn--c2br7g xn--cck2b3b xn--cckwcxetd xn--cg4bki xn--clchc0ea0b2g2a9gcd
xn--czr694b xn--czrs0t xn--czru2d xn--d1acj3b xn--d1alf xn--e1a4c xn--eckvdtc9d xn--efvy88h
xn--fct429k xn--fhbei xn--fiq228c5hs xn--fiq64b xn--fiqs8s xn--fiqz9s xn--fjq720a xn--flw351e
xn--fpcrj9c3d xn--fzc2c9e2c xn--fzys8d69uvgm xn--g2xx48c xn--gckr3f0f xn--gecrj9c xn--gk3at1e
xn--h2breg3eve xn--h2brj9c xn--h2brj9c8c xn--hxt814e xn--i1b6b1a6a2e xn--imr513n xn--io0a7i
xn--j1aef xn--j1amh xn--j6w193g xn--jlq480n2rg xn--jvr189m xn--kcrx77d1x4a xn--kprw13d xn--kpry57d
xn--kput3i xn--l1acc xn--lgbbat1ad8j xn--mgb9awbf xn--mgba3a3ejt xn--mgba3a4f16a xn--mgba7c0bbn0a
xn--mgbaam7a8h xn--mgbab2bd xn--mgbah1a3hjkrd xn--mgbai9azgqp6j xn--mgbayh7gpa xn--mgbbh1a
xn--mgbbh1a71e xn--mgbc0a9azcg xn--mgbca7dzdo xn--mgbcpq6gpa1a xn--mgberp4a5d4ar xn--mgbgu82a
xn--mgbi4ecexp xn--mgbpl2fh xn--mgbt3dhd xn--mgbtx2b xn--mgbx4cd0ab xn--mix891f xn--mk1bu44c
xn--mxtq1m xn--ngbc5azd xn--ngbe9e0a xn--ngbrx xn--node xn--nqv7f xn--nqv7fs00ema xn--nyqy26a
xn--o3cw4h xn--ogbpf8fl xn--otu796d xn--p1acf xn--p1ai xn--pgbs0dh xn--pssy2u xn--q7ce6a xn--q9jyb4c
xn--qcka1pmc xn--qxa6a xn--qxam xn--rhqv96g xn--rovu88b xn--rvc1e0am3e xn--s9brj9c xn--ses554g
xn--t60b56a xn--tckwe xn--tiq49xqyj xn--unup4y xn--vermgensberater-ctb xn--vermgensberatung-pwb
xn--vhquv xn--vuq861b xn--w4r85el8fhu5dnra xn--w4rs40l xn--wgbh1c xn--wgbl6a xn--xhq521b
xn--xkc2al3hye2a xn--xkc2dl3a5ee0h xn--y9a3aq xn--yfro4i67o xn--ygbi2ammx xn--zfr164b xxx xyz yachts
yahoo yamaxun yandex ye yodobashi yoga yokohama you youtube yt yun za zappos zara zero zip zm zone
zuerich zw
`.trim().split(/\s+/);

/** Every `xn--` name in `IANA_TLDS` beside its Unicode form, the form a field holds. */
export const IDN_TLDS: ReadonlyArray<readonly [string, string]> = [
  ["xn--11b4c3d", "\u0915\u0949\u092e"],
  ["xn--1ck2e1b", "\u30bb\u30fc\u30eb"],
  ["xn--1qqw23a", "\u4f5b\u5c71"],
  ["xn--2scrj9c", "\u0cad\u0cbe\u0cb0\u0ca4"],
  ["xn--30rr7y", "\u6148\u5584"],
  ["xn--3bst00m", "\u96c6\u56e2"],
  ["xn--3ds443g", "\u5728\u7ebf"],
  ["xn--3e0b707e", "\ud55c\uad6d"],
  ["xn--3hcrj9c", "\u0b2d\u0b3e\u0b30\u0b24"],
  ["xn--3pxu8k", "\u70b9\u770b"],
  ["xn--42c2d9a", "\u0e04\u0e2d\u0e21"],
  ["xn--45br5cyl", "\u09ad\u09be\u09f0\u09a4"],
  ["xn--45brj9c", "\u09ad\u09be\u09b0\u09a4"],
  ["xn--45q11c", "\u516b\u5366"],
  ["xn--4dbrk0ce", "\u05d9\u05e9\u05e8\u05d0\u05dc"],
  ["xn--4gbrim", "\u0645\u0648\u0642\u0639"],
  ["xn--54b7fta0cc", "\u09ac\u09be\u0982\u09b2\u09be"],
  ["xn--55qw42g", "\u516c\u76ca"],
  ["xn--55qx5d", "\u516c\u53f8"],
  ["xn--5su34j936bgsg", "\u9999\u683c\u91cc\u62c9"],
  ["xn--5tzm5g", "\u7f51\u7ad9"],
  ["xn--6frz82g", "\u79fb\u52a8"],
  ["xn--6qq986b3xl", "\u6211\u7231\u4f60"],
  ["xn--80adxhks", "\u043c\u043e\u0441\u043a\u0432\u0430"],
  ["xn--80ao21a", "\u049b\u0430\u0437"],
  ["xn--80aqecdr1a", "\u043a\u0430\u0442\u043e\u043b\u0438\u043a"],
  ["xn--80asehdb", "\u043e\u043d\u043b\u0430\u0439\u043d"],
  ["xn--80aswg", "\u0441\u0430\u0439\u0442"],
  ["xn--8y0a063a", "\u8054\u901a"],
  ["xn--90a3ac", "\u0441\u0440\u0431"],
  ["xn--90ae", "\u0431\u0433"],
  ["xn--90ais", "\u0431\u0435\u043b"],
  ["xn--9dbq2a", "\u05e7\u05d5\u05dd"],
  ["xn--9et52u", "\u65f6\u5c1a"],
  ["xn--9krt00a", "\u5fae\u535a"],
  ["xn--b4w605ferd", "\u6de1\u9a6c\u9521"],
  ["xn--bck1b9a5dre4c", "\u30d5\u30a1\u30c3\u30b7\u30e7\u30f3"],
  ["xn--c1avg", "\u043e\u0440\u0433"],
  ["xn--c2br7g", "\u0928\u0947\u091f"],
  ["xn--cck2b3b", "\u30b9\u30c8\u30a2"],
  ["xn--cckwcxetd", "\u30a2\u30de\u30be\u30f3"],
  ["xn--cg4bki", "\uc0bc\uc131"],
  ["xn--clchc0ea0b2g2a9gcd", "\u0b9a\u0bbf\u0b99\u0bcd\u0b95\u0baa\u0bcd\u0baa\u0bc2\u0bb0\u0bcd"],
  ["xn--czr694b", "\u5546\u6807"],
  ["xn--czrs0t", "\u5546\u5e97"],
  ["xn--czru2d", "\u5546\u57ce"],
  ["xn--d1acj3b", "\u0434\u0435\u0442\u0438"],
  ["xn--d1alf", "\u043c\u043a\u0434"],
  ["xn--e1a4c", "\u0435\u044e"],
  ["xn--eckvdtc9d", "\u30dd\u30a4\u30f3\u30c8"],
  ["xn--efvy88h", "\u65b0\u95fb"],
  ["xn--fct429k", "\u5bb6\u96fb"],
  ["xn--fhbei", "\u0643\u0648\u0645"],
  ["xn--fiq228c5hs", "\u4e2d\u6587\u7f51"],
  ["xn--fiq64b", "\u4e2d\u4fe1"],
  ["xn--fiqs8s", "\u4e2d\u56fd"],
  ["xn--fiqz9s", "\u4e2d\u570b"],
  ["xn--fjq720a", "\u5a31\u4e50"],
  ["xn--flw351e", "\u8c37\u6b4c"],
  ["xn--fpcrj9c3d", "\u0c2d\u0c3e\u0c30\u0c24\u0c4d"],
  ["xn--fzc2c9e2c", "\u0dbd\u0d82\u0d9a\u0dcf"],
  ["xn--fzys8d69uvgm", "\u96fb\u8a0a\u76c8\u79d1"],
  ["xn--g2xx48c", "\u8d2d\u7269"],
  ["xn--gckr3f0f", "\u30af\u30e9\u30a6\u30c9"],
  ["xn--gecrj9c", "\u0aad\u0abe\u0ab0\u0aa4"],
  ["xn--gk3at1e", "\u901a\u8ca9"],
  ["xn--h2breg3eve", "\u092d\u093e\u0930\u0924\u092e\u094d"],
  ["xn--h2brj9c", "\u092d\u093e\u0930\u0924"],
  ["xn--h2brj9c8c", "\u092d\u093e\u0930\u094b\u0924"],
  ["xn--hxt814e", "\u7f51\u5e97"],
  ["xn--i1b6b1a6a2e", "\u0938\u0902\u0917\u0920\u0928"],
  ["xn--imr513n", "\u9910\u5385"],
  ["xn--io0a7i", "\u7f51\u7edc"],
  ["xn--j1aef", "\u043a\u043e\u043c"],
  ["xn--j1amh", "\u0443\u043a\u0440"],
  ["xn--j6w193g", "\u9999\u6e2f"],
  ["xn--jlq480n2rg", "\u4e9a\u9a6c\u900a"],
  ["xn--jvr189m", "\u98df\u54c1"],
  ["xn--kcrx77d1x4a", "\u98de\u5229\u6d66"],
  ["xn--kprw13d", "\u53f0\u6e7e"],
  ["xn--kpry57d", "\u53f0\u7063"],
  ["xn--kput3i", "\u624b\u673a"],
  ["xn--l1acc", "\u043c\u043e\u043d"],
  ["xn--lgbbat1ad8j", "\u0627\u0644\u062c\u0632\u0627\u0626\u0631"],
  ["xn--mgb9awbf", "\u0639\u0645\u0627\u0646"],
  ["xn--mgba3a3ejt", "\u0627\u0631\u0627\u0645\u0643\u0648"],
  ["xn--mgba3a4f16a", "\u0627\u06cc\u0631\u0627\u0646"],
  ["xn--mgba7c0bbn0a", "\u0627\u0644\u0639\u0644\u064a\u0627\u0646"],
  ["xn--mgbaam7a8h", "\u0627\u0645\u0627\u0631\u0627\u062a"],
  ["xn--mgbab2bd", "\u0628\u0627\u0632\u0627\u0631"],
  ["xn--mgbah1a3hjkrd", "\u0645\u0648\u0631\u064a\u062a\u0627\u0646\u064a\u0627"],
  ["xn--mgbai9azgqp6j", "\u067e\u0627\u06a9\u0633\u062a\u0627\u0646"],
  ["xn--mgbayh7gpa", "\u0627\u0644\u0627\u0631\u062f\u0646"],
  ["xn--mgbbh1a", "\u0628\u0627\u0631\u062a"],
  ["xn--mgbbh1a71e", "\u0628\u06be\u0627\u0631\u062a"],
  ["xn--mgbc0a9azcg", "\u0627\u0644\u0645\u063a\u0631\u0628"],
  ["xn--mgbca7dzdo", "\u0627\u0628\u0648\u0638\u0628\u064a"],
  ["xn--mgbcpq6gpa1a", "\u0627\u0644\u0628\u062d\u0631\u064a\u0646"],
  ["xn--mgberp4a5d4ar", "\u0627\u0644\u0633\u0639\u0648\u062f\u064a\u0629"],
  ["xn--mgbgu82a", "\u0680\u0627\u0631\u062a"],
  ["xn--mgbi4ecexp", "\u0643\u0627\u062b\u0648\u0644\u064a\u0643"],
  ["xn--mgbpl2fh", "\u0633\u0648\u062f\u0627\u0646"],
  ["xn--mgbt3dhd", "\u0647\u0645\u0631\u0627\u0647"],
  ["xn--mgbtx2b", "\u0639\u0631\u0627\u0642"],
  ["xn--mgbx4cd0ab", "\u0645\u0644\u064a\u0633\u064a\u0627"],
  ["xn--mix891f", "\u6fb3\u9580"],
  ["xn--mk1bu44c", "\ub2f7\ucef4"],
  ["xn--mxtq1m", "\u653f\u5e9c"],
  ["xn--ngbc5azd", "\u0634\u0628\u0643\u0629"],
  ["xn--ngbe9e0a", "\u0628\u064a\u062a\u0643"],
  ["xn--ngbrx", "\u0639\u0631\u0628"],
  ["xn--node", "\u10d2\u10d4"],
  ["xn--nqv7f", "\u673a\u6784"],
  ["xn--nqv7fs00ema", "\u7ec4\u7ec7\u673a\u6784"],
  ["xn--nyqy26a", "\u5065\u5eb7"],
  ["xn--o3cw4h", "\u0e44\u0e17\u0e22"],
  ["xn--ogbpf8fl", "\u0633\u0648\u0631\u064a\u0629"],
  ["xn--otu796d", "\u62db\u8058"],
  ["xn--p1acf", "\u0440\u0443\u0441"],
  ["xn--p1ai", "\u0440\u0444"],
  ["xn--pgbs0dh", "\u062a\u0648\u0646\u0633"],
  ["xn--pssy2u", "\u5927\u62ff"],
  ["xn--q7ce6a", "\u0ea5\u0eb2\u0ea7"],
  ["xn--q9jyb4c", "\u307f\u3093\u306a"],
  ["xn--qcka1pmc", "\u30b0\u30fc\u30b0\u30eb"],
  ["xn--qxa6a", "\u03b5\u03c5"],
  ["xn--qxam", "\u03b5\u03bb"],
  ["xn--rhqv96g", "\u4e16\u754c"],
  ["xn--rovu88b", "\u66f8\u7c4d"],
  ["xn--rvc1e0am3e", "\u0d2d\u0d3e\u0d30\u0d24\u0d02"],
  ["xn--s9brj9c", "\u0a2d\u0a3e\u0a30\u0a24"],
  ["xn--ses554g", "\u7f51\u5740"],
  ["xn--t60b56a", "\ub2f7\ub137"],
  ["xn--tckwe", "\u30b3\u30e0"],
  ["xn--tiq49xqyj", "\u5929\u4e3b\u6559"],
  ["xn--unup4y", "\u6e38\u620f"],
  ["xn--vermgensberater-ctb", "verm\u00f6gensberater"],
  ["xn--vermgensberatung-pwb", "verm\u00f6gensberatung"],
  ["xn--vhquv", "\u4f01\u4e1a"],
  ["xn--vuq861b", "\u4fe1\u606f"],
  ["xn--w4r85el8fhu5dnra", "\u5609\u91cc\u5927\u9152\u5e97"],
  ["xn--w4rs40l", "\u5609\u91cc"],
  ["xn--wgbh1c", "\u0645\u0635\u0631"],
  ["xn--wgbl6a", "\u0642\u0637\u0631"],
  ["xn--xhq521b", "\u5e7f\u4e1c"],
  ["xn--xkc2al3hye2a", "\u0b87\u0bb2\u0b99\u0bcd\u0b95\u0bc8"],
  ["xn--xkc2dl3a5ee0h", "\u0b87\u0ba8\u0bcd\u0ba4\u0bbf\u0baf\u0bbe"],
  ["xn--y9a3aq", "\u0570\u0561\u0575"],
  ["xn--yfro4i67o", "\u65b0\u52a0\u5761"],
  ["xn--ygbi2ammx", "\u0641\u0644\u0633\u0637\u064a\u0646"],
  ["xn--zfr164b", "\u653f\u52a1"],
];
