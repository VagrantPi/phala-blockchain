const fs = require('fs');
const BN = require("bn.js");
const bigJson = require('big-json');

function normalizeHex(str) {
    return str.startsWith('0x') ? str : '0x' + str;
}

function praseBn(str) {
    let s = str.replace(/,/g, '');
    return new BN(s);
}

function loadJson(path) {
    const data = fs.readFileSync(path, {'encoding': 'utf-8'});
    return JSON.parse(data);
}

function loadJsonPromise(path) {
    return new Promise((resolve) => {
        const readStream = fs.createReadStream(path);
        const parseStream = bigJson.createParseStream();
        
        parseStream.on('data', function(pojo) {
            resolve(pojo)
        });

        readStream.pipe(parseStream);
    })
}

function writeJson(path, obj) {
    const data = JSON.stringify(obj, undefined, 2);
    fs.writeFileSync(path, data, {encoding: 'utf-8'});
}

module.exports = { normalizeHex, praseBn, loadJson, loadJsonPromise, writeJson };
