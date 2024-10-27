const board = document.getElementById('board');
const ws = new WebSocket('ws://localhost:3000/ws');
let currentPlayer = 'black';

function createBoard() {
    for (let i = 0; i < 64; i++) {
        const cell = document.createElement('div');
        cell.className = 'cell';
        cell.dataset.index = i;
        cell.addEventListener('click', () => makeMove(i));
        board.appendChild(cell);
    }
}

function updateBoard(gameState) {
    const cells = document.querySelectorAll('.cell');
    currentPlayer = gameState.turn;

    cells.forEach((cell, index) => {
        cell.innerHTML = '';
        cell.classList.remove('valid-move');
        if (gameState.black.includes(index)) {
            const piece = document.createElement('div');
            piece.className = 'piece black';
            cell.appendChild(piece);
        } else if (gameState.white.includes(index)) {
            const piece = document.createElement('div');
            piece.className = 'piece white';
            cell.appendChild(piece);
        }
        if (gameState.moves.includes(index)) {
            cell.classList.add('valid-move');
        }
    });
}

function makeMove(index) {
    const cell = document.querySelector(`.cell[data-index="${index}"]`);
    const playerType = document.getElementById(`${currentPlayer}-player`).value;

    if (cell.classList.contains('valid-move') && playerType === 'human') {
        ws.send(JSON.stringify({
            human_move: index
        }));
    }
}

function undoMove() {
    ws.send(JSON.stringify({ "undo": null }));
}

function redoMove() {
    ws.send(JSON.stringify({ "redo": null }));
}

function newGame() {
    ws.send(JSON.stringify({ "new_game": null }));
}

function xotGame() {
    ws.send(JSON.stringify({ "xot_game": null }));
}

ws.onmessage = (event) => {
    const gameState = JSON.parse(event.data);
    updateBoard(gameState);
};

createBoard();

board.addEventListener('contextmenu', (e) => {
    e.preventDefault(); // Prevent the default context menu
    undoMove();
});

document.getElementById('new-game-btn').addEventListener('click', newGame);
document.getElementById('xot-game-btn').addEventListener('click', xotGame);
document.getElementById('undo-btn').addEventListener('click', undoMove);
document.getElementById('redo-btn').addEventListener('click', redoMove);

document.getElementById('black-player').addEventListener('change', (e) => {
    ws.send(JSON.stringify({
        set_black_player: e.target.value
    }));
});

document.getElementById('white-player').addEventListener('change', (e) => {
    ws.send(JSON.stringify({
        set_white_player: e.target.value
    }));
});
