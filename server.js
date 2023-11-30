const games = {};

const createGame = () => {
  const newGameId = `game_${Date.now()}`;
  games[newGameId] = {
    players: [null, null],
    currentPlayer: 1,
    scores: [0, 0],
    dice: {
      player1: [4, 6, 8, 10, 20].map(rollDie),
      player2: [4, 6, 8, 10, 20].map(rollDie),
    },
  };
  return newGameId;
};

const rollDie = size => ({
  size: size,
  value: Math.ceil(Math.random() * size),
});

const performAttack = (gameId, playerId, attackerDieIndices, defenderDieIndex) => {
  console.log('performAttack', gameId, playerId, attackerDieIndices, defenderDieIndex);
  const game = games[gameId];
  if (!game) {
    return { message: 'Game not found' };
  }

  const currentPlayerIndex = game.players.indexOf(playerId);
  const currentPlayer = `player${currentPlayerIndex + 1}`;
  if (game.currentPlayer !== currentPlayerIndex + 1) {
    return { message: 'It is not your turn' };
  }

  const attackerDice = game.dice[currentPlayer];
  const opponentPlayer = `player${game.currentPlayer % 2 + 1}`;
  const defenderDice = game.dice[opponentPlayer];
  console.log('defenderDice', defenderDice);

  // Perform power attack or skill attack based on the number of attacker dice indices
  let attackSuccess = false;
  if (attackerDieIndices.length === 1) {
    // Power attack
    const attackerDie = attackerDice[attackerDieIndices[0]];
    if (attackerDie.value > defenderDice[defenderDieIndex].value) {
      attackSuccess = true;
      game.scores[currentPlayerIndex] += defenderDice[defenderDieIndex].size;
      defenderDice.splice(defenderDieIndex, 1); // Capture the die
    }
  } else {
    // Skill attack
    const attackValue = attackerDieIndices.reduce((acc, index) => acc + attackerDice[index].value, 0);
    console.log('defenderDieIndex', defenderDieIndex);
    console.log('defenderDice[defenderDieIndex]', defenderDice[defenderDieIndex]);
    if (attackValue === defenderDice[defenderDieIndex].value) {
      attackSuccess = true;
      game.scores[currentPlayerIndex] += defenderDice[defenderDieIndex].size;
      defenderDice.splice(defenderDieIndex, 1); // Capture the die
    }
  }

  // Check win condition
  if (defenderDice.length === 0) {
    return { message: `${currentPlayer} wins with a score of ${game.scores[currentPlayerIndex]}` };
  }

  // If attack successful, switch players
  if (attackSuccess) {
    game.currentPlayer = game.currentPlayer % 2 + 1;
  }

  return { message: attackSuccess ? 'Attack successful' : 'Attack failed', game };
};

const getGameStatus = gameId => {
  const game = games[gameId];
  if (!game) {
    return { message: 'Game not found' };
  }

  return game;
}

function joinGame(gameId, playerId) {
  // Check if the game exists
  const game = games[gameId];
  if (!game) {
    return { error: 'Game not found' };
  }

  // Check if the player has already joined
  if (game.players.includes(playerId)) {
    return { error: 'Player already joined' };
  }

  // Find an empty slot for the player
  const playerIndex = game.players.indexOf(null);
  if (playerIndex === -1) {
    return { error: 'Game is full' };
  }

  // Assign the player to the game
  game.players[playerIndex] = playerId;

  // Update the game state
  games[gameId] = game;

  return { message: `Player ${playerId} joined game ${gameId} as Player ${playerIndex + 1}` };
}

Bun.serve({
  async fetch(request) {
    const url = new URL(request.url);
    const gameId = url.pathname.split('/')[3];
    const action = url.pathname.split('/')[4];

    if (request.method === 'POST') {
      const playerId = request.headers.get('X-Player-Id');
      
      switch (action) {
        case undefined:
          const newGameId = createGame();
          return new Response(JSON.stringify({ gameId: newGameId }), { status: 200 });
        case 'join':
          return new Response(JSON.stringify(joinGame(gameId, playerId)), { status: 200 });
        case 'attack':
          const requestBody = await request.json();
          const { attackerDieIndices, defenderDieIndex } = requestBody;
          return new Response(JSON.stringify(performAttack(gameId, playerId, attackerDieIndices, defenderDieIndex)), { status: 200 });
        default:
          return new Response('Action not found', { status: 404 });
      }
    } else if (request.method === 'GET') {
      if (action === 'status') {
        return new Response(JSON.stringify(getGameStatus(gameId)), { status: 200 });
      }

      // Return the static files
      try {
        const fileUrl = new URL(url.pathname
            .replace(/^\//, './')
            .replace(/\/$/, '/index.html'),
          new URL('./dist/', new URL(import.meta.url)));
        const file = await Bun.file(fileUrl);
        return new Response(file);
      } catch (e) {
        console.error('Error', e);
        return new Response('Not found', { status: 404 });
      }
    } else {
      return new Response('Method not allowed', { status: 405 });
    }
  }
});