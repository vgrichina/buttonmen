import React, { useState, useEffect } from 'react';
import Cookies from 'js-cookie';

class APIError extends Error {
  constructor(message, code, data) {
    super(message);
    this.code = code;
    this.data = data;
  }

  toString() {
    return `${this.message} code: ${this.code} data: ${JSON.stringify(this.data)}`;
  }
}

const apiRequest = async (url, method, body) => {
  const response = await fetch(url, {
    method,
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(body),
  });

  // Handle errors
  if (!response.ok) {
    // TODO: Handle non-JSON errors
    const errorData = await response.json();
    throw new APIError('API request failed', response.status, errorData);
  }

  return response.json();
};

const get = async (url) => apiRequest(url, 'GET');
const post = async (url, body) => apiRequest(url, 'POST', body);

const playerId = Cookies.get('web4_account_id');
const contractId = window._web4Config?.contractName;

// TODO: Other dice types
const die = (size) => ({ kind: "Normal", size });

const Dice = ({ value, size }) => (
  <div>
    D{size}: {value}
  </div>
);

const joinGame = async (gameId) => {
  await post(`/web4/contract/${contractId}/join_game`, {
    game_id: gameId,
    // TODO: Let player choose dice set
    starting_dice: [die(4), die(6), die(8), die(10), die(20)],
  });
  window.location.href = `/games/${gameId}`;
};

const GameList = ({ games }) => (!games
  ? <div>Loading...</div>
  : <ul>
    {games.map(game => (
      <li key={game.id}>
        Game {game.id}: {game.players[0]} vs {game.players[1] || '???'} {
          game.players.some(p => p == playerId)
            ? <a href={`/games/${game.id}`}>Resume</a>
            : (game.players.some(p => p == "")
              ? <button onClick={() => joinGame(game.id)}>Join</button>
              : <a href={`/games/${game.id}`}>Spectate</a>)
        }
      </li>
    ))}
  </ul>
);

const usePolling = (deps, url, intervalMs = 2000) => {
  const [data, setData] = useState(null);

  useEffect(() => {
    const load = async () => {
      const data = await get(url);
      setData(data);
    }

    load();

    const interval = setInterval(load, intervalMs);
    return () => clearInterval(interval);
  }, [...deps, url, intervalMs]);

  return data;
};


const LatestGamesList = () => {
  const openGames = usePolling([], '/api/games');

  return (
    <div>
      <h2>Latest games created</h2>
      <GameList games={openGames} />
    </div>
  );
};

const AwaitingTurnGamesList = ({ gameId }) => {
  const games = usePolling([playerId], `/api/users/${playerId}/games`);
  const filteredGames = games?.filter(game => game.id !== gameId && game.current_player == game.players.indexOf(playerId));

  if (!filteredGames?.length) {
    return null;
  }

  return (
    <div>
      <h2>Awaiting your turn</h2>
      <GameList games={filteredGames} />
    </div>
  );
};

const Game = ({ gameId }) => {
  const [selectedDice, setSelectedDice] = useState([]); // To store indices of selected dice for an attack
  const [selectedDefenderDie, setSelectedDefenderDie] = useState(null); // Index of selected defender die

  // Progress indicators
  const [attacking, setAttacking] = useState(false);
  const [passing, setPassing] = useState(false);

  const gameState = usePolling([gameId], `/api/games/${gameId}/status`);

  const attack = async (attackerDieIndices, defenderDieIndex) => {
    await post(`/web4/contract/${contractId}/attack`, { game_id: gameId, attacker_die_indices: attackerDieIndices, defender_die_index: defenderDieIndex });
  };

  const pass = async () => {
    try {
      setPassing(true);
      await post(`/web4/contract/${contractId}/pass`, { game_id: gameId });
    } catch (e) {
      console.error(e);
      alert('Pass failed');
    } finally {
      setPassing(false);
    }
  }

  const selectDieForAttack = (index) => {
    setSelectedDice(prev => {
      // Add or remove the die index from the selection
      if (prev.includes(index)) {
        return prev.filter(i => i !== index);
      } else {
        return [...prev, index];
      }
    });
  };

  const selectDefenderDieForAttack = (index) => {
    setSelectedDefenderDie(index);
  };

  const performAttack = async () => {
    if (selectedDice.length === 0) {
      alert('Select at least one die to attack');
      return;
    }

    try {
      setAttacking(true);
      await attack(selectedDice, selectedDefenderDie);
    } catch (e) {
      console.error(e);
      alert('Attack failed');
    } finally {
      setAttacking(false);
    }

    // Reset selection after attack
    setSelectedDice([]);
    setSelectedDefenderDie(null);
  };

  const renderDice = (playerDice, dicePlayerId, isActive, captured) => (
    <div style={isActive ? { backgroundColor: 'rgb(255,247,230)' } : {}} >
      <h3>{dicePlayerId} {dicePlayerId == playerId && '(You)'}</h3>
      { isActive && dicePlayerId == playerId && <p><b>It's your turn</b></p> }
      <h4>Dice</h4>
      {playerDice.map((die, index) => {
        const isSelected = !isActive
          ? index === selectedDefenderDie
          : selectedDice.includes(index);
        return (
          <button
            key={index}
            onClick={() => !isActive ? selectDefenderDieForAttack(index) : selectDieForAttack(index)}
            style={{ backgroundColor: isSelected ? 'rgb(128,191,255)' : 'var(--button-base)' }}
          >
            <Dice value={die.value} size={die.size} />
          </button>
        );
      })}
      <h4>Captured</h4>
      <p>{captured.length > 0 ? captured.map((die) => `D${die}`).join(', ') : 'None'}</p>
      <p>Score: {captured.reduce((a, b) => a + b, 0)}</p>
    </div>
  );

  if (!gameState) {
    return <div>Loading...</div>;
  }

  const currentPlayerIndex = gameState.players.indexOf(playerId);
  const otherPlayerIndex = (currentPlayerIndex + 1) % 2;

  return (
    <div>
      <h2>{gameState.players[0]} playing against {gameState.players[1]}</h2>
      <div className="this-player">
        {renderDice(gameState.dice[currentPlayerIndex], gameState.players[currentPlayerIndex], gameState.current_player === currentPlayerIndex, gameState.captured[currentPlayerIndex])}
      </div>
      <div className="other-player">
        {gameState.players[otherPlayerIndex] == '' ? <p><b>Waiting for player to join...</b></p>
          : renderDice(gameState.dice[otherPlayerIndex], gameState.players[otherPlayerIndex], gameState.current_player === otherPlayerIndex, gameState.captured[otherPlayerIndex])}
      </div>

      {attacking && <p>Attacking...</p>}
      {!attacking && <button onClick={performAttack} disabled={gameState.players[gameState.current_player] !== playerId || gameState.is_pass_allowed}>Attack</button>}
      {passing && <p>Passing...</p>}
      {!passing && <button onClick={pass} disabled={gameState.players[gameState.current_player] !== playerId || !gameState.is_pass_allowed}>Pass</button>}

      <AwaitingTurnGamesList gameId={gameId} />
    </div>
  );
};

const LoggedInBanner = () => (
  <p><a href="/">Home</a> | Logged in as {playerId} | <a href="/web4/logout">Logout</a></p>
);

const RulesBanner = () => (
  <div>
    <p>See <a href="https://buttonmen.fandom.com/wiki/Button_Men_Rules#The_Basics">Button Men Rules</a> for the rules of the game.</p>
  </div>
);

const App = () => {
  if (!playerId) {
    return (
      <div className="App">
        <h1>Login to play</h1>
        <a href="/web4/login">Login</a>

        <RulesBanner />
      </div>
    );
  }

  const [creatingGame, setCreatingGame] = useState(false);

  const path = window.location.pathname;
  const parts = path.split('/');

  const createGame = async () => {
    setCreatingGame(true);
    try {
      const gameId = await post(`/web4/contract/${contractId}/create_game`, {
        // TODO: Let player choose dice set
        starting_dice: [die(4), die(6), die(8), die(10), die(20)],
      });

      console.log('Created game', gameId);
      // TODO: Push state to history instead?
      window.location.href = `/games/${gameId}`;
    } catch (e) {
      console.error(e);
      alert('Failed to create game');
    } finally {
      setCreatingGame(false);
    }
  };

  if (path === '/') {
    return <>
      <LoggedInBanner />
      {creatingGame && <p>Creating game...</p>}
      {!creatingGame && <button onClick={createGame}>Create game</button>}
      <LatestGamesList />

      <AwaitingTurnGamesList />

      <RulesBanner />
    </>
  }

  if (path.startsWith('/games/')) {
    const gameId = parts[2];
    return <>
      <LoggedInBanner />
      <Game gameId={gameId} />

      <RulesBanner />
    </>
  }

  // Redirect to homepage for unknown paths
  window.location.href = '/';
};

export default App;
